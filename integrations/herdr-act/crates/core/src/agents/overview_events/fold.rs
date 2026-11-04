//! Fold lifecycle signals while keeping turn completion separate from completed assignments.
use super::{Activity, Event, Pending};
use crate::agents::overview::PhaseRecord;

pub fn apply(activity: &mut Activity, event: &Event) {
    let fresh=event.at>=activity.last_seen;
    activity.key.clone_from(&event.key);
    activity.provider.clone_from(&event.provider);
    activity.session.clone_from(&event.session);
    if fresh && event.pane_id.is_some() {activity.pane_id.clone_from(&event.pane_id);}
    if fresh && event.core_terminal.is_some() {activity.core_terminal.clone_from(&event.core_terminal);}
    if fresh && !event.cwd.is_empty() {
        activity.cwd.clone_from(&event.cwd);
        let context = super::context::resolve(&event.cwd, "");
        activity.project = context.project;
    }
    if let Some(parent) = &event.parent { activity.parent = Some(parent.clone()); }
    if fresh {
        let seen = event.workdir.iter().chain([&event.cwd]).find_map(|d| super::worktree::linked(std::path::Path::new(d)));
        if let Some(tree) = seen { activity.worktree = Some(tree.path); }
    }
    if let Some(title) = event.title.as_ref().filter(|t| !t.is_empty() && fresh) {
        if activity.title.is_none() || (event.kind == "work" && activity.complete==Some(true)) { activity.title = Some(title.clone()); }
    }
    if let Some(ticket) = event.ticket.as_ref().filter(|_|fresh) { activity.ticket = Some(ticket.clone()); }
    // Skill files are read for briefing too; only subagents treat a read as entering that ACT command.
    let briefing = event.via == "SKILL.md" && event.parent.is_none() && activity.parent.is_none();
    if let Some(command) = event.command.as_ref().filter(|_| !briefing) {
        if event.phase_end {
            if let Some(record) = activity.phases.iter_mut().rev().find(|p| p.command.as_deref() == Some(command) && p.recorded_at<=event.at) {
                record.ended_at = Some(event.at);
            }
        } else if let Some(phase) = &event.phase {
            let last = activity.phases.last();
            let duplicate = last.is_some_and(|p| {
                let same_turn = event.turn_id.is_some() && p.turn_id == event.turn_id;
                p.command.as_ref() == Some(command) && p.ended_at.is_none()
                    && ((p.invocation.is_some() && p.invocation == event.tool_id)
                        || (same_turn && p.source == "UserPromptSubmit" && event.via == "PreToolUse")
                        || event.via == "SKILL.md")
            });
            if !duplicate {
                activity.phases.push(PhaseRecord {
                    phase: phase.clone(), recorded_at: event.at, command: Some(command.clone()),
                    source: event.via.clone(), invocation: event.tool_id.clone().or_else(|| event.turn_id.clone()),
                    turn_id: event.turn_id.clone(), ended_at: None,
                });
            }
            activity.phases.sort_by_key(|p|p.recorded_at);
            activity.phase=activity.phases.last().map(|p|p.phase.clone());
        }
    }
    if event.at < activity.last_seen { return; }
    activity.last_seen = event.at;
    let id = event.tool_id.clone().unwrap_or_else(|| "reported".into());
    match event.kind.as_str() {
        "work" | "child_start" => {
            activity.complete = Some(false);
            activity.generation = Some(event.at);
            activity.pending.clear();
            activity.error = None;
            activity.runtime = "working".into();
        }
        "working" => {
            if event.turn_id.is_some() && activity.turn_id.is_some() && event.turn_id != activity.turn_id {
                activity.complete = Some(false);
                activity.generation = Some(event.at);
                activity.pending.clear();
            }
            activity.error = None;
            activity.runtime = "working".into();
        }
        "question" | "async_question" | "approval" => {
            activity.pending.insert(id, Pending { kind: event.kind.clone(), text: event.text.clone().unwrap_or_default() });
            activity.runtime = "waiting".into();
        }
        "resolved" => {
            if !activity.pending.get(&id).is_some_and(|p|p.kind=="async_question") {activity.pending.remove(&id);}
            activity.error = None; activity.runtime = "working".into();
        }
        "idle" | "waiting-parent" => {
            activity.pending.retain(|_, p| matches!(p.kind.as_str(),"reported_question"|"async_question"));
            activity.runtime = if event.kind=="waiting-parent" {"waiting-parent"} else {"idle"}.into();
        }
        "closed" | "interrupted" => { activity.pending.clear(); activity.runtime = event.kind.clone(); }
        "error" => { activity.pending.remove(&id); activity.error.clone_from(&event.text); activity.runtime = "error".into(); }
        "report" => {
            if event.phase.is_some() && !event.phase_end {
                activity.runtime="working".into();
                if activity.complete!=Some(false) {activity.generation=Some(event.at);}
                activity.complete=Some(false);
            }
            if let Some(question) = &event.text {
                activity.pending.insert("reported".into(), Pending { kind: "reported_question".into(), text: question.clone() });
            }
        }
        _ => {}
    }
    if event.complete.is_some() {
        if event.complete==Some(false) {activity.generation=Some(event.at);}
        activity.complete = event.complete;
        if event.complete == Some(true) { activity.pending.clear(); }
    }
    if event.turn_id.is_some() { activity.turn_id.clone_from(&event.turn_id); }
}
