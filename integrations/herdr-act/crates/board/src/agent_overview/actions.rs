//! Keyboard actions and saved corrections for the agent overview.
use super::*;

impl App {
    pub(super) fn apply_corrections(&mut self) {
        let mut changed = false;
        for session in &mut self.sessions {
            changed |= self.store.apply(session);
        }
        if changed {
            if let Err(error) = self.store.save() { self.error = Some(error); }
        }
    }

    pub(super) fn rows(&self) -> Vec<overview_tree::Row<'_>> {
        overview_tree::rows(&self.sessions).into_iter().filter(|row| {
            if self.project.as_ref().is_some_and(|p| &row.session.project != p) { return false; }
            if self.show_hidden { return true; }
            let mut node=Some(row.session);
            for _ in 0..=self.sessions.len() {
                let Some(session)=node else {break};
                if session.hidden {return false;}
                node=session.parent_id.as_ref().and_then(|id|self.sessions.iter().find(|s|&s.id==id));
            }
            true
        }).collect()
    }
    pub(super) fn visible(&self) -> Vec<&OverviewSession> {
        self.rows().into_iter().filter(|row| {
            if self.collapsed_groups.contains(&group_order(row.group)) {return false;}
            let mut parent=row.session.parent_id.as_deref();
            for _ in 0..self.sessions.len() {
                let Some(id)=parent else {break};
                if self.collapsed.contains(id) {return false;}
                parent=self.sessions.iter().find(|s|s.id==id).and_then(|s|s.parent_id.as_deref());
            }
            true
        }).map(|row|row.session).collect()
    }
    pub(super) fn current(&self) -> Option<&OverviewSession> {
        self.sessions.get(self.selected).filter(|s|self.visible().iter().any(|v|v.id==s.id))
    }
    pub(super) fn ensure_selection(&mut self) {
        if self.current().is_none() {
            self.selected=self.visible().first().and_then(|s|self.sessions.iter().position(|all|all.id==s.id)).unwrap_or(0);
            self.terminal_text.clear();
            self.detail_scroll=0;
        }
    }

    pub(super) fn move_selection(&mut self, amount: isize) {
        let visible = self.visible();
        if visible.is_empty() { return; }
        let current = visible.iter().position(|session| session.id == self.sessions[self.selected].id).unwrap_or(0);
        let next = (current as isize + amount).rem_euclid(visible.len() as isize) as usize;
        if let Some(index) = self.sessions.iter().position(|session| session.id == visible[next].id) { self.selected = index; }
        self.terminal_text.clear();
        self.detail_scroll=0;
        self.refresh_preview();
    }

    pub(super) fn next_project(&mut self) {
        let projects = self.sessions.iter().filter(|s| self.show_hidden || !s.hidden)
            .map(|s| s.project.clone()).filter(|p| p != "Unknown").collect::<BTreeSet<_>>();
        let options = std::iter::once(None).chain(projects.into_iter().map(Some)).collect::<Vec<_>>();
        let current = options.iter().position(|p| p == &self.project).unwrap_or(0);
        self.project = options[(current + 1) % options.len()].clone();
        self.selected = self.visible().first().and_then(|s| self.sessions.iter().position(|all| all.id == s.id)).unwrap_or(0);
        self.terminal_text.clear();
        self.detail_scroll=0;
        self.refresh_preview();
    }

    pub(super) fn toggle_completion(&mut self) {
        let Some(session) = self.current().cloned() else { return };
        let correction = self.store.correction(&session.id);
        correction.manual_completion = match correction.manual_completion {None=>Some(true),Some(true)=>Some(false),Some(false)=>None};
        correction.completion_generation = session.activity_generation;
        if let Err(error) = self.store.save() { self.error = Some(error); return; }
        self.apply_corrections();
        self.sort_sessions();
    }

    pub(super) fn toggle_hidden(&mut self) {
        let Some(session) = self.current().cloned() else { return };
        let family_done=self.rows().iter().find(|r|r.session.id==session.id).is_some_and(|r|r.group==OverviewGroup::Done);
        if session.hidden {
            let correction = self.store.correction(&session.id);
            correction.hidden = false;
            correction.hidden_generation = None;
        } else if session.manual_completion.or(session.task_complete) == Some(true) && (session.parent_id.is_some() || family_done) {
            self.store.hide(&session);
        } else {
            self.error = Some("Hide is available only for completed work.".into());
            return;
        }
        if let Err(error) = self.store.save() { self.error = Some(error); return; }
        self.apply_corrections();
    }

    pub(super) fn edit_selected(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<(), String> {
        let Some(session) = self.current().cloned() else { return Ok(()) };
        ratatui::restore();
        print!("Enter title|project|ticket|ACT phase (empty keeps, - clears): ");
        let _ = std::io::stdout().flush();
        let mut input = String::new();
        let read = std::io::stdin().read_line(&mut input);
        *terminal = ratatui::init();
        read.map_err(|e| e.to_string())?;
        let fields = input.trim().splitn(4, '|').collect::<Vec<_>>();
        if fields.len() != 4 {
            self.error = Some("Expected four fields separated by |.".into());
            return Ok(());
        }
        {
            let correction = self.store.correction(&session.id);
            set_optional(&mut correction.title, fields[0]);
            set_optional(&mut correction.project, fields[1]);
            set_optional(&mut correction.ticket, fields[2]);
        }
        if fields[3] == "-" {
            self.store.correction(&session.id).act_phase = None;
        } else if !fields[3].trim().is_empty() {
            let phase = fields[3].trim().to_owned();
            self.store.record_phase(
                &session.id,
                phase,
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            );
        }
        self.store.save()?;
        self.apply_corrections();
        self.sort_sessions();
        Ok(())
    }

    pub(super) fn open_selected(&mut self) {
        let Some(session) = self.current().cloned() else { return };
        match &session.source {
            SessionSource::Native {terminal} => {
                if let Err(error)=std::process::Command::new("herdr-app").args(["do","agent.focus","--terminal",terminal]).status() {self.error=Some(error.to_string());}
            },
            SessionSource::Ended => self.error=Some("Die Agentensitzung ist beendet. Der abgeschlossene Auftrag bleibt hier erhalten.".into()),
            SessionSource::Herdr { target, .. } => {
                let parent = env_val("HERDR_PANE_ID").unwrap_or_default();
                if !parent.is_empty() {
                    match self.herdr.pane_split(&parent, &session.project_path, "right", &[]) {
                        Ok(pane) => {
                            let pane_id = pane.s("pane_id").to_owned();
                            if !pane_id.is_empty() {
                                let command = format!("herdr agent attach {}", shell_arg(target));
                                match self.herdr.pane_run(&pane_id, &command) {
                                    Ok(_) => {
                                        if let Some(old)=self.attached_pane.replace(pane_id) {let _=self.herdr.pane_close(&old);}
                                    },
                                    Err(error) => {let _=self.herdr.pane_close(&pane_id); self.error=Some(error.message);},
                                }
                            }
                        }
                        Err(error) => self.error = Some(error.message),
                    }
                } else if let Err(error) = self.herdr.agent_focus(target) {
                    self.error = Some(error.message);
                }
            }
            SessionSource::CodexApp { thread_id } => {
                let url = format!("codex://threads/{thread_id}");
                #[cfg(target_os = "macos")]
                let opened = std::process::Command::new("/usr/bin/open").arg(url).spawn();
                #[cfg(target_os = "linux")]
                let opened = std::process::Command::new("xdg-open").arg(url).spawn();
                match opened {
                    Err(error)=>self.error=Some(error.to_string()),
                    Ok(_)=>if let Some(old)=self.attached_pane.take() {let _=self.herdr.pane_close(&old);},
                }
            }
        }
    }
}

fn shell_arg(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn set_optional(target: &mut Option<String>, input: &str) {
    let input = input.trim();
    if input == "-" {
        *target = None;
    } else if !input.is_empty() {
        *target = Some(input.to_owned());
    }
}
