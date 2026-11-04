//! Durable agent lifecycle signals shared by Codex, Claude Code, and ACT.
pub mod context;
pub mod fold;
pub mod install;
pub mod normalize;
pub mod project;
pub mod scope;
pub mod shell;
pub mod worktree;
use super::overview::{OverviewSession, PhaseRecord};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::{self, OpenOptions}, os::unix::{fs::{OpenOptionsExt, PermissionsExt}, io::AsRawFd}, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Event {
    pub key: String,
    pub provider: String,
    pub session: String,
    pub cwd: String,
    pub pane_id: Option<String>,
    pub core_terminal: Option<String>,
    pub at: u64,
    pub kind: String,
    pub via: String,
    pub parent: Option<String>,
    pub title: Option<String>,
    pub tool_id: Option<String>,
    pub turn_id: Option<String>,
    pub text: Option<String>,
    pub command: Option<String>,
    pub phase: Option<String>,
    pub phase_end: bool,
    pub ticket: Option<String>,
    pub complete: Option<bool>,
    pub workdir: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pending {
    pub kind: String,
    pub text: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Activity {
    pub key: String,
    pub provider: String,
    pub session: String,
    pub cwd: String,
    pub pane_id: Option<String>,
    pub core_terminal: Option<String>,
    pub project: String,
    pub title: Option<String>,
    pub parent: Option<String>,
    pub runtime: String,
    pub pending: BTreeMap<String, Pending>,
    pub error: Option<String>,
    pub phase: Option<String>,
    pub phases: Vec<PhaseRecord>,
    pub ticket: Option<String>,
    pub complete: Option<bool>,
    pub generation: Option<u64>,
    pub turn_id: Option<String>,
    /// Last linked git worktree the session worked in; Codex subagents keep the root cwd and pass it as workdir.
    pub worktree: Option<String>,
    pub last_seen: u64,
}
pub fn session_key(provider: &str, id: &str) -> String {
    format!("{}:{id}", if provider == "codex" { "codex-thread" } else { "claude-session" })
}
pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
pub fn directory() -> PathBuf { crate::store::files::data_dir().join("agent-overview/sessions") }
pub fn record(event: &Event) -> Result<(), String> { record_at(&directory(), event) }
pub fn record_at(dir: &Path, event: &Event) -> Result<(), String> {
    if !matches!(event.provider.as_str(), "codex" | "claude") || event.session.is_empty() || event.session.len()>256
        || event.key != session_key(&event.provider, &event.session) {
        return Err("Invalid agent event identity".into());
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    let hash = sha1_smol::Sha1::from(event.key.as_bytes()).digest().to_string();
    let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).mode(0o600)
        .open(dir.join(format!("{hash}.lock"))).map_err(|e| e.to_string())?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let path = dir.join(format!("{hash}.json"));
    let mut activity: Activity = match fs::read_to_string(&path) {
        Ok(text)=>serde_json::from_str(&text).map_err(|e|format!("Invalid activity file {}: {e}",path.display()))?,
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Activity::default(),
        Err(e)=>return Err(e.to_string()),
    };
    fold::apply(&mut activity, event);
    let value = serde_json::to_value(&activity).map_err(|e| e.to_string())?;
    crate::store::files::write_json(&path, &value).map_err(|e| e.to_string())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    // Closing the file releases flock even when a write above fails.
    Ok(())
}
pub fn load_at(dir: &Path) -> Result<Vec<Activity>, String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.to_string()),
    };
    let mut activities = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|s| s == "json") {
            let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            activities.push(serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?);
        }
    }
    Ok(activities)
}
pub fn enrich(sessions: &mut Vec<OverviewSession>,manual:impl Fn(&str)->Option<bool>) -> Result<(), String> {
    let activities=load_at(&directory())?;
    project::apply(sessions,&activities);
    if project::retain_completed(sessions,&activities,manual) {project::apply(sessions,&activities);}
    Ok(())
}
