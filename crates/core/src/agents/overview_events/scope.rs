//! Limit CLI observation to Herdr and recognize Codex Desktop from its own session metadata.
use crate::json::Value;
use std::{fs,io::{BufRead,BufReader,Read},path::Path};

fn desktop(value:&str)->bool {matches!(value,"Codex Desktop"|"codex_work_desktop")}
pub fn native_terminal()->Option<String> {
    let terminal=std::env::var("HC_TERMINAL_ID").ok().filter(|s|!s.is_empty() && s.len()<128)?;
    let mut address=terminal.clone();
    for _ in 0..8 {
        let output=std::process::Command::new("herdr-core").args(["get",&address,"--json"]).output().ok()?;
        if !output.status.success() {return None;}
        let value:Value=serde_json::from_slice(&output.stdout).ok()?;
        if value["labels"]["herdr.space"].as_str().is_some() {return Some(terminal);}
        address=value["parent"].as_str()?.to_owned();
    }
    None
}
pub fn native_parent(terminal:&str)->Option<String> {
    let output=std::process::Command::new("herdr-core").args(["get",terminal,"--json"]).output().ok()?;
    if !output.status.success() {return None;}
    let value:Value=serde_json::from_slice(&output.stdout).ok()?;
    let parent=value["parent"].as_str()?;
    let activities=super::load_at(&super::directory()).ok()?;
    Some(activities.iter().filter(|a|a.parent.is_none() && a.core_terminal.as_deref()==Some(parent))
        .max_by_key(|a|a.last_seen).map(|a|a.key.clone()).unwrap_or_else(||format!("core-terminal:{parent}")))
}
pub fn allows(provider:&str,input:&Value,managed:bool,codex_home:&Path,signals:&Path)->bool {
    if managed {return true;}
    if provider=="codex" {
        if input["originator"].as_str().is_some_and(desktop) {return true;}
        if let Some(path)=input["transcript_path"].as_str().and_then(|s|Path::new(s).canonicalize().ok()) {
            let roots=[codex_home.join("sessions"),codex_home.join("archived_sessions")];
            if roots.iter().filter_map(|p|p.canonicalize().ok()).any(|root|path.starts_with(root)) {
                if let Ok(file)=fs::File::open(path) {
                    let mut first=String::new();
                    let _=BufReader::new(file.take(512_000)).read_line(&mut first);
                    if serde_json::from_str::<Value>(&first).ok().and_then(|v|v["payload"]["originator"].as_str().map(desktop))==Some(true) {return true;}
                }
            }
        }
    }
    let Some(session)=input["session_id"].as_str() else {return false;};
    let key=super::session_key(provider,session);
    let hash=sha1_smol::Sha1::from(key.as_bytes()).digest().to_string();
    let saved=super::Activity::default();
    let activity:super::Activity=fs::read(signals.join(format!("{hash}.json"))).ok()
        .and_then(|v|serde_json::from_slice(&v).ok()).unwrap_or(saved);
    // Detached helper callbacks can lack the parent's terminal environment.
    activity.parent.is_some() || (input["agent_id"].as_str().is_some() && !activity.key.is_empty())
}
