//! Live Herdr and Codex App sessions, with runtime status kept apart from task completion.
use crate::json::{Js, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseRecord {
    pub phase: String,
    pub recorded_at: u64,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub invocation: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub ended_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum OverviewGroup {
    NeedYou,
    Problems,
    Active,
    Done,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SessionSource {
    Herdr { target: String, pane_id: String },
    CodexApp { thread_id: String },
    Native { terminal: String },
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OverviewSession {
    pub id: String,
    pub title: String,
    pub program: String,
    pub preview: String,
    pub project: String,
    pub project_path: String,
    pub ticket: Option<String>,
    pub act_phase: Option<String>,
    pub phase_history: Vec<PhaseRecord>,
    pub manual_completion: Option<bool>,
    pub task_complete: Option<bool>,
    pub question: Option<String>,
    pub needs_user: bool,
    pub problem: Option<String>,
    pub phase_source: Option<String>,
    pub hidden: bool,
    pub runtime_status: String,
    pub group: OverviewGroup,
    pub completion_seq: Option<u64>,
    pub activity_generation: Option<u64>,
    pub parent_id: Option<String>,
    pub source: SessionSource,
    /// The linked git worktree the session works in, or for a coordinator the one its latest subagent works in.
    pub worktree: Option<super::overview_events::worktree::Worktree>,
}

pub fn from_herdr(agents: &[Value], workspaces: &[Value]) -> Vec<OverviewSession> {
    agents
        .iter()
        .map(|agent| {
            let pane_id = agent.s("pane_id").to_owned();
            let session = agent.at("agent_session");
            let stable_id = session.so("value").map(|value| {
                if session.s("agent") == "codex" {
                    format!("codex-thread:{value}")
                } else {
                    if matches!(session.s("agent"), "claude" | "claude-code" | "claude_code") { format!("claude-session:{value}") }
                        else { format!("herdr:{}:{value}", session.s("source")) }
                }
            });
            let workspace = workspaces
                .iter()
                .find(|workspace| workspace.s("workspace_id") == agent.s("workspace_id"));
            let mut project = workspace
                .and_then(|workspace| {
                    workspace.at("worktree").so("repo_name")
                        .or_else(|| workspace.so("label"))
                        .or_else(|| workspace.at("worktree").so("repo_root"))
                        .or_else(|| workspace.so("cwd"))
                })
                .unwrap_or("Unknown")
                .to_owned();
            let project_path = workspace
                .and_then(|workspace| {
                    [
                        workspace.at("worktree").so("checkout_path"),
                        workspace.at("worktree").so("repo_root"),
                        workspace.so("cwd"),
                    ]
                        .iter()
                        .find_map(|path| *path)
                })
                .unwrap_or_default()
                .to_owned();
            if workspace.and_then(|w|w.at("worktree").so("repo_name")).is_none() {
                let derived=super::overview_events::context::resolve(&project_path,"").project;
                if !derived.is_empty() {project=derived;}
            }
            let title = [
                agent.so("name"),
                agent.so("title"),
                agent.so("terminal_title_stripped"),
                agent.so("display_agent"),
                agent.so("agent"),
            ]
            .into_iter()
            .flatten()
            .next()
            .unwrap_or("Untitled Herdr session")
            .to_owned();
            let runtime_status = agent.s("agent_status").to_owned();
            let completion_seq = agent.at("completion_seq").as_u64();
            OverviewSession {
                id: stable_id.unwrap_or_else(|| format!("herdr-pane:{pane_id}")),
                title,
                program: match session.so("agent").or(agent.so("agent")).unwrap_or("").to_lowercase().replace([' ','_','-'],"").as_str() {
                    "claudecode"=>"claude".into(),"codexcli"=>"codex".into(),other=>other.to_owned(),
                },
                preview: String::new(),
                project,
                project_path,
                ticket: None,
                act_phase: None,
                phase_history: Vec::new(),
                worktree: None,
                manual_completion: None,
                task_complete: None,
                question: None,
                needs_user: false,
                problem: None,
                phase_source: None,
                hidden: false,
                group: herdr_group(&runtime_status, completion_seq),
                runtime_status,
                completion_seq,
                activity_generation: agent.at("state_change_seq").as_u64(),
                parent_id: None,
                source: SessionSource::Herdr {
                    target: pane_id.clone(),
                    pane_id,
                },
            }
        })
        .collect()
}

pub fn from_codex_app(threads: &[Value]) -> Vec<OverviewSession> {
    let mut accepted = threads.iter().filter(|t| matches!(t.s("originator"), "Codex Desktop" | "codex_work_desktop"))
        .map(|t| t.s("id").to_owned()).collect::<std::collections::BTreeSet<_>>();
    loop {
        let before = accepted.len();
        for thread in threads {
            if thread.so("parentThreadId").is_some_and(|id| accepted.contains(id)) { accepted.insert(thread.s("id").to_owned()); }
        }
        if accepted.len() == before { break; }
    }
    threads.iter().filter(|thread| accepted.contains(thread.s("id")))
        .map(|thread| {
            let thread_id = thread.s("id").to_owned();
            let status = thread.at("status");
            let runtime_status = status.s("type").to_owned();
            let project_path = thread.so("cwd").unwrap_or("").to_owned();
            let active_flags = status
                .arr("activeFlags")
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            let group = codex_group(&runtime_status, &active_flags);
            OverviewSession {
                id: format!("codex-thread:{thread_id}"),
                title: thread
                    .so("name")
                    .unwrap_or("Untitled Codex thread")
                    .to_owned(),
                program:"codex".into(),
                preview: thread.so("preview").unwrap_or("").to_owned(),
                project: project_name(&project_path),
                project_path,
                ticket: None,
                act_phase: None,
                phase_history: Vec::new(),
                worktree: None,
                manual_completion: None,
                task_complete: None,
                question: None,
                needs_user: group == OverviewGroup::NeedYou,
                problem: None,
                phase_source: None,
                hidden: false,
                runtime_status,
                group,
                completion_seq: None,
                activity_generation: thread.at("updatedAt").as_u64(),
                parent_id: thread.so("parentThreadId").map(|id| format!("codex-thread:{id}")),
                source: SessionSource::CodexApp { thread_id },
            }
        })
        .collect()
}

fn project_name(path: &str) -> String {
    let context = super::overview_events::context::resolve(path, "");
    if !context.project.is_empty() { return context.project; }
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Unknown")
        .to_owned()
}

fn herdr_group(status: &str, _completion_seq: Option<u64>) -> OverviewGroup {
    match status {
        "blocked" | "error" | "failed" => OverviewGroup::Problems,
        "working" => OverviewGroup::Active,
        _ => OverviewGroup::Unknown,
    }
}
fn codex_group(status: &str, active_flags: &[&str]) -> OverviewGroup {
    if active_flags.iter().any(|flag| {
        matches!(*flag, "waitingOnApproval" | "waitingOnUserInput" | "waitingOnPermissions")
    }) {
        OverviewGroup::NeedYou
    } else {
        match status {
            "systemError" => OverviewGroup::Problems,
            "active" => OverviewGroup::Active,
            _ => OverviewGroup::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json;

    #[test]
    fn herdr_runtime_and_completion_are_independent() {
        let agents = vec![
            json!({"terminal_id":"t1","pane_id":"p1","workspace_id":"w1","agent_status":"idle"}),
            json!({"terminal_id":"t2","pane_id":"p2","workspace_id":"w1","agent_status":"idle","completion_seq":3}),
        ];
        let sessions = from_herdr(&agents, &[]);
        assert_eq!(sessions[0].group, OverviewGroup::Unknown);
        assert_eq!(sessions[0].runtime_status, "idle");
        assert_eq!(sessions[1].group, OverviewGroup::Unknown);
        assert_eq!(sessions[1].runtime_status, "idle");
    }

    #[test]
    fn codex_waiting_and_system_errors_get_separate_groups() {
        let threads = vec![
            json!({"id":"a","originator":"Codex Desktop","status":{"type":"active","activeFlags":["waitingOnApproval"]}}),
            json!({"id":"b","originator":"Codex Desktop","status":{"type":"systemError"}}),
            json!({"id":"c","originator":"codex_work_desktop","status":{"type":"idle"}}),
            json!({"id":"d","originator":"codex-tui","status":{"type":"active"}}),
        ];
        let sessions = from_codex_app(&threads);
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].group, OverviewGroup::NeedYou);
        assert_eq!(sessions[1].group, OverviewGroup::Problems);
        assert_eq!(sessions[2].group, OverviewGroup::Unknown);
    }
}
