//! Read saved Codex child conversations and name them from their recorded task identity, without starting a client.
use super::{adapter, codex_app_cache, homes::codex_home, overview::OverviewSession, read_turn::{find_file, head_of, tail_lines}, talk};
use crate::{err::{Error, Res}, json::{Js, Value}, store::files};
use serde_json::json;
use std::{collections::{BTreeMap, BTreeSet}, fs, os::unix::fs::PermissionsExt, path::Path};

fn thread_id(key: &str) -> Res<&str> {
    let id = key.strip_prefix("codex-thread:").unwrap_or("");
    if id.len() != 36 || !id.bytes().enumerate().all(|(i, c)| {
        if [8, 13, 18, 23].contains(&i) { c == b'-' } else { c.is_ascii_hexdigit() }
    }) { return Err(Error::new("Ungültige Codex-Gesprächskennung.")); }
    Ok(id)
}
fn metadata(file: &Path) -> Option<Value> {
    head_of(file, 400_000).lines().filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|row| row.s("type") == "session_meta").map(|row| row["payload"].clone())
}
fn fallback(id: &str) -> String { format!("Unteragent {}", &id[..8]) }
fn generic(title: &str) -> bool { matches!(title.trim().to_lowercase().as_str(), "" | "default" | "agent" | "subagent" | "unteragent") }
fn readable(text: &str) -> bool {
    !(text.len() > 100 && text.starts_with("gAAAAA") && text.bytes().all(|c| c.is_ascii_alphanumeric() || b"_-=".contains(&c)))
}
fn title(meta: &Value, id: &str) -> String {
    let spawn = &meta["source"]["subagent"]["thread_spawn"];
    if let Some(leaf) = spawn.s("agent_path").rsplit('/').find(|part| !part.is_empty()) {
        if leaf != "root" && !generic(leaf) { return leaf.replace(['_', '-'], " ").chars().take(100).collect(); }
    }
    let nickname = meta.so("agent_nickname").or_else(|| spawn.so("agent_nickname"));
    nickname.filter(|name| !generic(name)).map(|name| format!("Unteragent {name}")).unwrap_or_else(|| fallback(id))
}
/// Index immutable task names once; refreshing the agent list must not parse every child's conversation.
pub fn label_children(sessions: &mut [OverviewSession]) -> Res<()> {
    let path = codex_app_cache::directory().join("thread-labels.json");
    let mut labels: BTreeMap<String, String> = fs::read(&path).ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    let wanted = sessions.iter().filter(|s| s.parent_id.is_some() && generic(&s.title))
        .filter_map(|s| thread_id(&s.id).ok()).filter(|id| !labels.contains_key(*id)).map(str::to_owned).collect::<BTreeSet<_>>();
    let before = labels.len();
    if !wanted.is_empty() {
        for file in find_file(&codex_home().join("sessions"), &|name| name.ends_with(".jsonl"), 4) {
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let Some(id) = stem.get(stem.len().saturating_sub(36)..).filter(|id| wanted.contains(*id)) else { continue };
            if let Some(meta) = metadata(&file).filter(|meta| meta.s("id") == id) { labels.insert(id.into(), title(&meta, id)); }
        }
    }
    for session in sessions.iter_mut().filter(|s| s.parent_id.is_some() && generic(&s.title)) {
        if let Ok(id) = thread_id(&session.id) { session.title = labels.get(id).cloned().unwrap_or_else(|| fallback(id)); }
    }
    if labels.len() != before {
        let directory = path.parent().unwrap();
        fs::create_dir_all(directory)?;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        files::write_json(&path, &serde_json::to_value(labels)?)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
fn assignment(meta: &Value) -> Option<String> {
    let spawn = &meta["source"]["subagent"]["thread_spawn"];
    let parent = spawn.s("parent_thread_id");
    thread_id(&format!("codex-thread:{parent}")).ok()?;
    let path = adapter::transcript("codex", parent, Path::new(meta.s("cwd")))?;
    let mut calls = BTreeMap::new();
    for row in tail_lines(&path, 6 * 1024 * 1024).ok()? {
        let payload = &row["payload"];
        if row.s("type") != "response_item" { continue; }
        if payload.s("type") == "function_call" && payload.s("name") == "spawn_agent" {
            if let Ok(args) = serde_json::from_str::<Value>(payload.s("arguments")) {
                if let Some(message) = args.so("message").filter(|text| readable(text)) { calls.insert(payload.t("call_id"), message.to_owned()); }
            }
        } else if payload.s("type") == "function_call_output" {
            if let Ok(result) = serde_json::from_str::<Value>(payload.s("output")) {
                if !spawn.s("agent_path").is_empty() && result.s("task_name") == spawn.s("agent_path") {
                    if let Some(message) = calls.get(payload.s("call_id")) { return Some(message.to_owned()); }
                }
            }
        }
    }
    None
}
pub fn read(key: &str) -> Res<Value> {
    let id = thread_id(key)?;
    let unavailable = || json!({"id":key,"available":false,"messages":[],"assignment":null,"watchThreads":[id]});
    let Some(path) = adapter::transcript("codex", id, Path::new("")) else { return Ok(unavailable()); };
    let Some(meta) = metadata(&path) else { return Ok(unavailable()); };
    if meta.s("id") != id { return Err(Error::new("Die Gesprächsdatei gehört zu einem anderen Agenten.")); }
    let Some(said) = talk::talk_of("codex", Some(&path), id, Path::new(meta.s("cwd")), 81) else { return Ok(unavailable()); };
    let mut truncated = said.len() > 80;
    let messages = said.iter().skip(said.len().saturating_sub(80))
        .filter(|m| readable(&m.text) && (!m.you || !(m.text.starts_with("# AGENTS.md instructions") || m.text.starts_with("<environment_context>"))))
        .map(|m| {
            let text: String = m.text.chars().take(32_000).collect();
            truncated |= text.len() < m.text.len();
            json!({"role":if m.you {"user"} else {"assistant"},"text":text,"at":m.at})
        }).collect::<Vec<_>>();
    let assignment = assignment(&meta).map(|text| {
        let clipped: String = text.chars().take(32_000).collect();
        truncated |= clipped.len() < text.len();
        clipped
    });
    let parent = meta["source"]["subagent"]["thread_spawn"].s("parent_thread_id").to_owned();
    let watch_threads: Vec<&str> = [id, parent.as_str()].into_iter().filter(|id| !id.is_empty()).collect();
    Ok(json!({"id":key,"available":true,"title":title(&meta,id),"assignment":assignment,"messages":messages,"truncated":truncated,"watchThreads":watch_threads}))
}
