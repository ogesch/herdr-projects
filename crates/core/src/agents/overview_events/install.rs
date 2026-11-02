//! Install additive lifecycle observers with immutable binaries and preserve existing hook trust and settings.
use crate::{agents::homes::{claude_home, codex_home}, json::Value, util::shq};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

const DESCRIPTION: &str = "Herdr Agent Overview lifecycle observer";

pub fn install() -> Result<Vec<PathBuf>, String> {
    let mut paths=install_at(&codex_home(), &claude_home(), &crate::store::files::data_dir(),
        &std::env::current_exe().map_err(|e| e.to_string())?)?;
    if let Some(path)=super::shell::install()? {paths.push(path);}
    Ok(paths)
}
pub fn install_at(codex: &Path, claude: &Path, data: &Path, executable: &Path) -> Result<Vec<PathBuf>, String> {
    let bytes = fs::read(executable).map_err(|e| e.to_string())?;
    let digest = sha1_smol::Sha1::from(&bytes).digest().to_string();
    let binary = data.join("agent-overview/bin").join(digest).join("herdr-projects");
    fs::create_dir_all(binary.parent().unwrap()).map_err(|e| e.to_string())?;
    if !binary.exists() {
        crate::store::files::write_atomic(&binary, &bytes).map_err(|e| e.to_string())?;
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    } else if fs::read(&binary).map_err(|e|e.to_string())?!=bytes {
        return Err("Installed immutable observer binary has changed; refusing to overwrite it".into());
    }
    let inline = fs::read_to_string(codex.join("config.toml")).ok()
        .and_then(|s| s.parse::<toml::Value>().ok()).is_some_and(|v| v.get("hooks").is_some());
    if inline { return Err("Codex has inline TOML hooks. Merge the observer into that layer before installing hooks.json.".into()); }
    let mut planned = Vec::new();
    for (provider, path) in [("codex", codex.join("hooks.json")), ("claude", claude.join("settings.json"))] {
        let mut config: Value = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("{} is invalid JSON: {e}", path.display()))?
        } else { json!({}) };
        let original = config.clone();
        let root = config.as_object_mut().ok_or("Hook settings must be an object")?;
        let hooks = root.entry("hooks").or_insert_with(|| json!({})).as_object_mut().ok_or("hooks must be an object")?;
        let mut events = vec!["SessionStart","UserPromptSubmit","PreToolUse","PermissionRequest","PostToolUse","SubagentStart","SubagentStop","Stop","SessionEnd"];
        if provider == "codex" { events.push("Interrupt"); }
        else { events.extend(["PostToolUseFailure","StopFailure","Notification"]); }
        let command = format!("env {} {} agents hook {provider}", shq(&format!("HERDR_PROJECTS_DATA_DIR={}", data.display())), shq(&binary.display().to_string()));
        for event in events {
            let groups = hooks.entry(event).or_insert_with(|| json!([])).as_array_mut().ok_or("Hook event must be an array")?;
            groups.retain(|group| group["description"].as_str() != Some(DESCRIPTION));
            groups.push(json!({"description":DESCRIPTION,"hooks":[{"type":"command","command":command,"timeout":3}]}));
        }
        if config == original { continue; }
        planned.push((path,config,original));
    }
    let mut changed=Vec::new();
    for (path,config,original) in planned {
        let current=if path.exists() {serde_json::from_slice::<Value>(&fs::read(&path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?} else {json!({})};
        if current!=original {return Err(format!("{} changed during installation; retry after reviewing it",path.display()));}
        if path.exists() {
            let backup = path.with_extension(format!("json.before-herdr-overview-{}", super::now_ms()));
            fs::copy(&path, &backup).map_err(|e| e.to_string())?;
            fs::set_permissions(backup, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        }
        crate::store::files::write_json(&path, &config).map_err(|e| e.to_string())?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
        changed.push(path);
    }
    Ok(changed)
}
