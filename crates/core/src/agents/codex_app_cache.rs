//! Keep the last Codex snapshot available while one background worker refreshes its slower daemon metadata.
use crate::{err::{Kind, Res}, json::Value, store::files};
use serde::{Deserialize, Serialize};
use std::{fs, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

pub const LOCK: &str = "codex-overview-refresh";
const REFRESH_MS: i64 = 30_000;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct Stored {
    threads: Vec<Value>,
    updated_at: Option<i64>,
    attempted_at: Option<i64>,
    error: Option<String>,
}
pub struct Snapshot {
    pub threads: Vec<Value>,
    pub updated_at: Option<i64>,
    pub error: Option<String>,
    pub refreshing: bool,
    pub refresh_due: bool,
}
pub fn directory() -> PathBuf { files::data_dir().join("agent-overview/codex-cache") }
fn stored(directory: &Path) -> Res<Stored> {
    match fs::read(directory.join("snapshot.json")) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Stored::default()),
        Err(error) => Err(error.into()),
    }
}
fn save(directory: &Path, record: &Stored) -> Res<()> {
    fs::create_dir_all(directory)?;
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    let path = directory.join("snapshot.json");
    files::write_atomic(&path, &serde_json::to_vec(record)?)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}
fn due(record: &Stored, now: i64) -> bool {
    record.attempted_at.is_none_or(|at| now.saturating_sub(at) >= REFRESH_MS)
}
pub fn read_at(directory: &Path, now: i64, refreshing: bool) -> Res<Snapshot> {
    let record = stored(directory)?;
    let refresh_due = !refreshing && due(&record, now);
    Ok(Snapshot { threads: record.threads, updated_at: record.updated_at, error: record.error, refreshing, refresh_due })
}
pub fn refresh_at(directory: &Path, now: impl Fn() -> i64, fetch: impl FnOnce() -> Result<Vec<Value>, String>) -> Res<bool> {
    let _held = match files::hold_lock(LOCK, 0) {
        Ok(held) => held,
        Err(error) if error.kind == Kind::Busy => return Ok(false),
        Err(error) => return Err(error),
    };
    let mut record = stored(directory)?;
    if !due(&record, now()) { return Ok(false); }
    record.attempted_at = Some(now());
    save(directory, &record)?;
    let result = fetch();
    record.attempted_at = Some(now());
    match result {
        Ok(threads) => { record.threads = threads; record.updated_at = Some(now()); record.error = None; }
        Err(error) => record.error = Some(error),
    }
    save(directory, &record)?;
    Ok(true)
}
