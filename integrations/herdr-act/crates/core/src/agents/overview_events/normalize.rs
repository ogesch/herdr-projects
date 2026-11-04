//! Normalize documented Codex and Claude lifecycle payloads without deciding permissions.
use super::{session_key, Event};
use crate::json::Value;
use regex::Regex;

pub fn phase(command: &str) -> Option<&'static str> {
    let command = command.trim().trim_start_matches(['/', '$']).rsplit(':').next()?;
    match command.strip_suffix("-flutter").unwrap_or(command) {
        "act-interview" => Some("Interview"),
        "act-create-spec" => Some("Spec erstellen"),
        "act-refine-spec" => Some("Refine"),
        "act-create-issues" => Some("Create Issues"),
        "act-implement" => Some("Implement"),
        "land-feature" | "act-land-feature" => Some("Land Feature"),
        "outside-act" => Some("Außerhalb ACT"),
        _ => None,
    }
}
fn command_in_prompt(prompt: &str) -> Option<String> {
    let pattern = Regex::new(r"(?m)^\s*(?:[/\$]|<command-name>/)(act-[a-z-]+|land-feature)(?:\s|$|</command-name>)").unwrap();
    pattern.captures(prompt).map(|c| c[1].to_owned()).filter(|c| phase(c).is_some())
}
/// Codex has no Skill tool: an ACT skill is entered by reading its SKILL.md through a shell or script tool.
fn command_in_skill_read(input: &Value) -> Option<String> {
    let pattern = Regex::new(r"skills/((?:act-[a-z-]+)|land-feature)/SKILL\.md").unwrap();
    pattern.captures_iter(&input.to_string()).map(|c| c[1].to_owned()).find(|c| phase(c).is_some())
}
fn prompt_text(prompt:&str)->&str {
    let trimmed=prompt.trim();
    if trimmed.starts_with("<hc from=") && trimmed.ends_with("</hc>") {
        if let Some((_,body))=trimmed.split_once('>') {return body.trim_end_matches("</hc>").trim();}
    }
    prompt
}
pub fn text(value: &Value) -> String {
    value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()).chars().take(4000).collect()
}
fn input_object(value: &Value) -> Value {
    if let Some(s) = value.as_str() { serde_json::from_str(s).unwrap_or_else(|_| value.clone()) }
    else { value.clone() }
}
fn question(input: &Value) -> String {
    let Some(questions) = input["questions"].as_array() else { return text(input); };
    questions.iter().take(8).map(|q| {
        let asked = q["question"].as_str().or(q["title"].as_str()).unwrap_or("Nutzereingabe erforderlich");
        let options = q["options"].as_array().into_iter().flatten().take(6)
            .filter_map(|o| o["label"].as_str().or(o.as_str())).collect::<Vec<_>>().join(" / ");
        if options.is_empty() { asked.to_owned() } else { format!("{asked} ({options})") }
    }).collect::<Vec<_>>().join("\n").chars().take(4000).collect()
}
pub fn from_hook(provider: &str, input: &Value, at: u64) -> Result<Vec<Event>, String> {
    if !matches!(provider, "codex" | "claude") { return Err("Expected codex or claude".into()); }
    let session = input["session_id"].as_str().filter(|s| !s.is_empty() && s.len() <= 256)
        .ok_or("Hook omitted session_id")?;
    let event_name = input["hook_event_name"].as_str().ok_or("Hook omitted hook_event_name")?;
    let tool = input["tool_name"].as_str().unwrap_or("");
    let tool_input = input_object(&input["tool_input"]);
    let mut event = Event {
        key: session_key(provider, session), provider: provider.into(), session: session.into(),
        cwd: input["cwd"].as_str().unwrap_or("").into(), at, via: event_name.into(),
        tool_id: input["tool_use_id"].as_str().or(input["tool_call_id"].as_str()).map(str::to_owned),
        turn_id: input["turn_id"].as_str().map(str::to_owned), ..Default::default()
    };
    event.workdir = tool_input["workdir"].as_str().or(tool_input["cwd"].as_str()).filter(|w| w.starts_with('/')).map(str::to_owned);
    if !matches!(event_name,"SubagentStart"|"SubagentStop") {
        if let Some(child)=input["agent_id"].as_str().filter(|id|*id!=session && !id.is_empty()) {
            event.parent=Some(event.key.clone());
            event.session=child.into();
            event.key=session_key(provider,child);
        }
    }
    event.kind = match event_name {
        "SessionStart" => "start",
        "UserPromptSubmit" => {
            let prompt = prompt_text(input["prompt"].as_str().unwrap_or(""));
            event.command = command_in_prompt(prompt);
            event.title = Some(prompt.lines().next().unwrap_or("").chars().take(90).collect());
            event.ticket = super::context::resolve(&event.cwd, prompt).ticket;
            "work"
        }
        "PreToolUse" => {
            if tool == "Skill" || tool.ends_with("load_skill") {
                event.command = tool_input["skill"].as_str().or(tool_input["name"].as_str())
                    .map(|s| s.trim_start_matches(['/', '$']).rsplit(':').next().unwrap_or(s).to_owned())
                    .filter(|c| phase(c).is_some());
                event.ticket=super::context::resolve(&event.cwd,tool_input["args"].as_str().unwrap_or("")).ticket;
            } else if let Some(command) = command_in_skill_read(&tool_input) {
                event.command = Some(command);
                event.via = "SKILL.md".into();
            }
            if tool.contains("AskUserQuestion") || tool.ends_with("request_user_input") || tool.ends_with("request_user_input_async") {
                event.text = Some(question(&tool_input));
                if tool.ends_with("request_user_input_async") {"async_question"} else {"question"}
            } else { "working" }
        }
        "PermissionRequest" => {
            event.text = Some(["description", "command", "cmd", "file_path", "reason"].iter()
                .find_map(|k| tool_input.get(k).filter(|v| !v.is_null()).map(text))
                .unwrap_or_else(|| format!("Freigabe für {tool}: {}", text(&tool_input))));
            "approval"
        }
        "PostToolUse" => {
            event.ticket=super::context::resolve(&event.cwd,tool_input["file_path"].as_str().unwrap_or("")).ticket;
            let response=input_object(&input["tool_response"]);
            let response=response.get("stdout").or(response.get("output")).map(input_object).unwrap_or(response);
            if let Some(title)=response["title"].as_str() {
                if let Some(url)=response["url"].as_str().filter(|u|u.starts_with("https://github.com/")) {
                    event.ticket=Some(format!("{url} · {}",title.chars().take(200).collect::<String>()));
                }
            }
            "resolved"
        },
        "PostToolUseFailure" | "StopFailure" => {
            event.text = Some(text(input.get("error").or(input.get("error_message")).unwrap_or(&input["tool_response"])));
            "error"
        }
        "SubagentStart" | "SubagentStop" => {
            let child = input["agent_id"].as_str().filter(|s| !s.is_empty()).ok_or("Subagent event omitted agent_id")?;
            event.parent = Some(event.key.clone());
            event.session = child.into();
            event.key = session_key(provider, child);
            event.title = input["agent_type"].as_str().map(str::to_owned);
            if event_name == "SubagentStart" { "child_start" } else { "waiting-parent" }
        }
        "Notification" if matches!(input["notification_type"].as_str(), Some("permission_prompt" | "elicitation_dialog")) => {
            event.text = Some(text(&input["message"])); "approval"
        }
        "Stop" => "idle",
        "Interrupt" => "interrupted",
        "SessionEnd" => "closed",
        _ => return Ok(vec![]),
    }.into();
    if event.tool_id.is_none() && !tool.is_empty() { event.tool_id = Some(tool.into()); }
    event.phase = event.command.as_deref().and_then(phase).map(str::to_owned);
    Ok(vec![event])
}
