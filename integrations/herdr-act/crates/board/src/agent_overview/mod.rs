//! A live Herdr and Codex Desktop overview beside the selected interactive session.
use crate::{
    agents::{codex_app, overview::{self, OverviewGroup, OverviewSession, SessionSource}, overview_events, overview_tree},
    agents::overview_store::OverviewStore, herdr::api::Herdr, json::Js, util::env_val,
};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style},
    text::{Line, Span, Text}, widgets::{Block, Borders, Paragraph}, Frame};
use std::{collections::BTreeSet, io::{IsTerminal, Write}, sync::mpsc, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

mod actions;
mod ui;
#[cfg(test)]
mod tests;
use ui::group_order;

pub fn open_overview() -> i32 {
    if env_val("HERDR_ENV").as_deref() != Some("1") {
        eprintln!("Open Agent Overview from a Herdr-managed pane."); return 1;
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("Agent Overview needs an interactive Herdr pane."); return 1;
    }
    let mut terminal = ratatui::init();
    let result = App::new().and_then(|mut app| app.run(&mut terminal));
    ratatui::restore();
    match result { Ok(()) => 0, Err(error) => { eprintln!("{error}"); 1 } }
}
struct App {
    herdr: Herdr,
    herdr_sessions: Vec<OverviewSession>,
    codex_sessions: Vec<OverviewSession>,
    sessions: Vec<OverviewSession>,
    store: OverviewStore,
    selected: usize,
    project: Option<String>,
    show_hidden: bool,
    collapsed: BTreeSet<String>,
    collapsed_groups: BTreeSet<u8>,
    expanded_history: BTreeSet<String>,
    detail_scroll: u16,
    terminal_text: String,
    error: Option<String>,
    codex_error: Option<String>,
    codex_rx: mpsc::Receiver<Result<Vec<crate::json::Value>, String>>,
    codex_tx: mpsc::Sender<Result<Vec<crate::json::Value>, String>>,
    codex_inflight: bool,
    next_herdr: Instant,
    next_codex: Instant,
    attached_pane: Option<String>,
}
impl App {
    fn new() -> Result<Self, String> {
        let (codex_tx, codex_rx) = mpsc::channel();
        let mut app = Self {
            herdr: Herdr::new(None), herdr_sessions: vec![], codex_sessions: vec![], sessions: vec![],
            store: OverviewStore::default(), selected: 0, project: None, show_hidden: false,
            collapsed: BTreeSet::new(), collapsed_groups: BTreeSet::new(), expanded_history:BTreeSet::new(), detail_scroll:0, terminal_text: String::new(),
            error: None, codex_error: None, codex_rx, codex_tx, codex_inflight: false,
            next_herdr: Instant::now(), next_codex: Instant::now(), attached_pane: None,
        };
        if let Some(error)=app.store.error() {return Err(error.into());}
        app.refresh_herdr()?;
        app.request_codex();
        Ok(app)
    }
    fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<(), String> {
        loop {
            self.refresh();
            terminal.draw(|frame| self.draw(frame)).map_err(|e| e.to_string())?;
            if event::poll(Duration::from_millis(150)).map_err(|e| e.to_string())? {
                if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                    if key.kind != KeyEventKind::Press { continue; }
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                        KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
                        KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
                        KeyCode::Char('p') => self.next_project(),
                        KeyCode::Char('a') => self.show_hidden = !self.show_hidden,
                        KeyCode::Char('e') => self.edit_selected(terminal)?,
                        KeyCode::Char('m') => self.toggle_completion(),
                        KeyCode::Char('h') => self.toggle_hidden(),
                        KeyCode::Char('w') => if let Some(session)=self.current() {
                            let id=session.id.clone();
                            if !self.expanded_history.remove(&id) {self.expanded_history.insert(id);}
                            self.detail_scroll=0;
                        },
                        KeyCode::PageUp => self.detail_scroll=self.detail_scroll.saturating_sub(8),
                        KeyCode::PageDown => self.detail_scroll=self.detail_scroll.saturating_add(8),
                        KeyCode::Char(' ') => if let Some(session) = self.current() {
                            let id = session.id.clone();
                            if !self.collapsed.remove(&id) { self.collapsed.insert(id); }
                        },
                        KeyCode::Char(c @ '1'..='5') => {
                            let group = c as u8 - b'1';
                            if !self.collapsed_groups.remove(&group) { self.collapsed_groups.insert(group); }
                        }
                        KeyCode::Char('r') => { self.next_herdr = Instant::now(); self.next_codex = Instant::now(); }
                        KeyCode::Enter => self.open_selected(),
                        _ => {}
                    }
                    self.ensure_selection();
                }
            }
        }
    }
    fn refresh(&mut self) {
        if Instant::now() >= self.next_herdr {
            if let Err(error) = self.refresh_herdr() { self.error = Some(error); }
        }
        if let Ok(result) = self.codex_rx.try_recv() {
            self.codex_inflight = false;
            self.next_codex = Instant::now() + Duration::from_secs(5);
            match result {
                Ok(threads) => {
                    self.codex_sessions = overview::from_codex_app(&threads);
                    self.codex_error = None;
                    self.rebuild();
                }
                Err(error) => self.codex_error = Some(error),
            }
        }
        if Instant::now() >= self.next_codex { self.request_codex(); }
    }
    fn rebuild(&mut self) {
        let selected_id = self.sessions.get(self.selected).map(|s| s.id.clone());
        self.sessions.clone_from(&self.herdr_sessions);
        let herdr_ids = self.herdr_sessions.iter().map(|s| &s.id).collect::<BTreeSet<_>>();
        self.sessions.extend(self.codex_sessions.iter().filter(|s| !herdr_ids.contains(&s.id)).cloned());
        if let Err(error) = overview_events::enrich(&mut self.sessions,|id|self.store.completion(id)) { self.error = Some(error); }
        self.apply_corrections();
        self.sort_sessions();
        if let Some(id) = selected_id {
            if let Some(index) = self.sessions.iter().position(|s| s.id == id) { self.selected = index; }
        }
        self.ensure_selection();
    }
    fn refresh_herdr(&mut self) -> Result<(), String> {
        self.next_herdr = Instant::now() + Duration::from_secs(2);
        let agents = self.herdr.agents().map_err(|e| e.message)?;
        let workspaces = self.herdr.workspaces().map_err(|e| e.message)?;
        self.herdr_sessions = overview::from_herdr(&agents, &workspaces);
        self.error = None;
        self.rebuild();
        self.refresh_preview();
        Ok(())
    }
    fn refresh_preview(&mut self) {
        self.terminal_text = self.current().and_then(|session| match &session.source {
            SessionSource::Herdr { pane_id, .. } => self.herdr.pane_read(pane_id, "recent", Some(35)).ok(),
            SessionSource::CodexApp { .. } => None,
            SessionSource::Ended => None,
            SessionSource::Native { .. } => None,
        }).unwrap_or_default();
    }
    fn request_codex(&mut self) {
        if self.codex_inflight { return; }
        self.codex_inflight = true;
        let tx = self.codex_tx.clone();
        thread::spawn(move || { let _ = tx.send(codex_app::list_threads()); });
    }
    fn sort_sessions(&mut self) {
        self.sessions.sort_by(|a,b| group_order(a.group).cmp(&group_order(b.group))
            .then_with(|| a.project.cmp(&b.project)).then_with(|| a.title.cmp(&b.title)));
        self.selected = self.selected.min(self.sessions.len().saturating_sub(1));
    }
}
