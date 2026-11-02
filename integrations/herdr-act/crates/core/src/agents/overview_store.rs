//! User corrections, assignment completion overrides, hidden state, and manual ACT history.
use super::overview::{OverviewSession, PhaseRecord};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCorrection {
    pub title: Option<String>,
    pub project: Option<String>,
    pub ticket: Option<String>,
    pub act_phase: Option<String>,
    pub phase_history: Vec<PhaseRecord>,
    pub manual_completion: Option<bool>,
    pub completion_generation: Option<u64>,
    pub hidden: bool,
    pub hidden_generation: Option<u64>,
    #[serde(default)]
    pub closed: bool,
    pub closed_generation: Option<u64>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoredCorrections {
    #[serde(default)]
    sessions: BTreeMap<String, SessionCorrection>,
}
#[derive(Debug, Clone)]
pub struct OverviewStore { file: PathBuf, sessions: BTreeMap<String, SessionCorrection>, load_error: Option<String> }
impl Default for OverviewStore {
    fn default() -> Self { Self::at(crate::store::files::data_dir().join("agent-overview.json")) }
}
impl OverviewStore {
    pub fn at(file: impl Into<PathBuf>) -> Self {
        let file = file.into();
        let read=match std::fs::read_to_string(&file) {
            Ok(text)=>serde_json::from_str::<StoredCorrections>(&text).map_err(|e|format!("Invalid overview corrections {}: {e}",file.display())),
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(StoredCorrections::default()),
            Err(e)=>Err(e.to_string()),
        };
        let (sessions,load_error)=match read {Ok(stored)=>(stored.sessions,None),Err(error)=>(BTreeMap::new(),Some(error))};
        Self { file, sessions, load_error }
    }
    pub fn error(&self)->Option<&str> {self.load_error.as_deref()}
    pub fn completion(&self,id:&str)->Option<bool> {self.sessions.get(id).and_then(|c|c.manual_completion)}
    pub fn correction(&mut self, id: &str) -> &mut SessionCorrection { self.sessions.entry(id.to_owned()).or_default() }
    pub fn record_phase(&mut self, id: &str, phase: String, recorded_at: u64) {
        let recorded_at=if recorded_at<1_000_000_000_000 {recorded_at*1000} else {recorded_at};
        let correction = self.correction(id);
        correction.act_phase = Some(phase.clone());
        correction.phase_history.push(PhaseRecord {
            phase, recorded_at, source: "manual".into(), command: None, invocation: None, turn_id: None, ended_at: None,
        });
    }
    pub fn hide(&mut self, session: &OverviewSession) {
        let correction = self.correction(&session.id);
        correction.hidden = true;
        correction.hidden_generation = session.activity_generation;
    }
    pub fn apply(&mut self, session: &mut OverviewSession) -> bool {
        let mut migrated=false;
        if session.parent_id.is_none() {
            let old_pane=match &session.source {
                super::overview::SessionSource::Herdr{pane_id,..}=>format!("herdr-pane:{pane_id}"),
                super::overview::SessionSource::Native{terminal}=>format!("core-terminal:{terminal}"),_=>String::new(),
            };
            if !old_pane.is_empty() {
                let suffix=session.id.strip_prefix("claude-session:").map(|s|format!(":{s}"));
                let old=self.sessions.keys().find(|key|**key!=session.id &&
                    (**key==old_pane || suffix.as_ref().is_some_and(|suffix|key.starts_with("herdr:")&&key.ends_with(suffix)))).cloned();
                if let Some(old)=old {
                    let legacy=self.sessions.remove(&old).unwrap();
                    let current=self.sessions.entry(session.id.clone()).or_insert_with(||legacy.clone());
                    if current.title.is_none() {current.title=legacy.title;}
                    if current.project.is_none() {current.project=legacy.project;}
                    if current.ticket.is_none() {current.ticket=legacy.ticket;}
                    if current.act_phase.is_none() {current.act_phase=legacy.act_phase;}
                    if current.manual_completion.is_none() {
                        current.manual_completion=legacy.manual_completion;
                        current.completion_generation=legacy.completion_generation;
                    }
                    for phase in legacy.phase_history {if !current.phase_history.contains(&phase) {current.phase_history.push(phase);}}
                    migrated=true;
                }
            }
        }
        let Some(correction) = self.sessions.get_mut(&session.id) else { return false };
        let new_work = session.task_complete == Some(false) || matches!(session.runtime_status.as_str(), "working" | "active");
        let newer = |saved| session.activity_generation.zip(saved).is_some_and(|(a, b)| a > b);
        let mut changed = migrated;
        if new_work && newer(correction.completion_generation) {
            correction.manual_completion = None;
            correction.completion_generation = None;
            changed = true;
        }
        if correction.hidden && new_work && newer(correction.hidden_generation) {
            correction.hidden = false;
            correction.hidden_generation = None;
            changed = true;
        }
        if correction.closed && new_work && (newer(correction.closed_generation)
            || (correction.closed_generation.is_none() && session.activity_generation.is_some())) {
            correction.closed = false;
            correction.closed_generation = None;
            correction.hidden = false;
            correction.hidden_generation = None;
            changed = true;
        }
        if let Some(title) = &correction.title { session.title.clone_from(title); }
        if let Some(project) = &correction.project { session.project.clone_from(project); }
        if correction.ticket.is_some() { session.ticket.clone_from(&correction.ticket); }
        if correction.act_phase.is_some() {
            session.act_phase.clone_from(&correction.act_phase);
            session.phase_source = Some("manual".into());
        }
        for phase in &correction.phase_history {
            if !session.phase_history.contains(phase) { session.phase_history.push(phase.clone()); }
        }
        session.phase_history.sort_by_key(|p| p.recorded_at);
        session.manual_completion = correction.manual_completion;
        session.hidden = correction.hidden;
        if correction.closed {
            session.runtime_status = "closed".into();
            session.source = super::overview::SessionSource::Ended;
            session.question = None;
            session.needs_user = false;
        }
        super::overview_events::project::classify(session);
        changed
    }
    pub fn save(&self) -> Result<(), String> {
        if let Some(error)=&self.load_error {return Err(error.clone());}
        let value = serde_json::to_value(StoredCorrections { sessions: self.sessions.clone() }).map_err(|e| e.to_string())?;
        crate::store::files::write_json(&self.file, &value).map_err(|e| e.to_string())
    }
}
