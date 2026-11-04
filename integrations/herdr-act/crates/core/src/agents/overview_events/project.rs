//! Join hook signals to known sessions and attach CLI subagents to their existing interactive parent.
use super::Activity;
use crate::agents::overview::{OverviewGroup, OverviewSession, SessionSource};
use std::collections::{BTreeSet,BTreeMap};

pub fn apply(sessions: &mut Vec<OverviewSession>, activities: &[Activity]) {
    let mut linked=BTreeMap::new();
    for session in sessions.iter_mut() {
        let old=session.id.clone();
        if session.id.starts_with("herdr-pane:") || session.id.starts_with("core-terminal:") {
            {
                if let Some(activity)=activities.iter().filter(|a|a.parent.is_none() && a.provider==session.program && match &session.source {
                    SessionSource::Herdr{pane_id,..}=>a.pane_id.as_ref()==Some(pane_id),
                    SessionSource::Native{terminal}=>a.core_terminal.as_ref()==Some(terminal),_=>false,
                })
                    .max_by_key(|a|a.last_seen) {session.id.clone_from(&activity.key);}
            }
        }
        if let Some(activity) = activities.iter().find(|a| a.key == session.id) { fields(session, activity, false); }
        if old!=session.id {linked.insert(old,session.id.clone());}
    }
    for session in sessions.iter_mut() {if let Some(parent)=session.parent_id.as_ref().and_then(|p|linked.get(p)) {session.parent_id=Some(parent.clone());}}
    let mut unique=BTreeSet::new();
    sessions.retain(|s|unique.insert(s.id.clone()));
    let mut seen = sessions.iter().map(|s| s.id.clone()).collect::<BTreeSet<_>>();
    loop {
        let before = sessions.len();
        for activity in activities {
            if seen.contains(&activity.key) { continue; }
            let Some(parent) = activity.parent.as_ref().and_then(|id| sessions.iter().find(|s| &s.id == id)).cloned() else { continue; };
            let mut child = parent;
            child.id.clone_from(&activity.key);
            child.title = activity.title.clone().unwrap_or_else(|| "Unteragent".into());
            child.preview.clear();
            child.manual_completion = None;
            child.task_complete = None;
            child.question = None;
            child.needs_user = false;
            child.problem = None;
            child.hidden = false;
            child.completion_seq = None;
            child.phase_history.clear();
            child.act_phase = None;
            child.phase_source = None;
            child.ticket = None;
            fields(&mut child, activity, true);
            seen.insert(child.id.clone());
            sessions.push(child);
        }
        if sessions.len() == before { break; }
    }
    for _ in 0..sessions.len() {
        let active=sessions.iter().filter(|s|matches!(s.group,OverviewGroup::NeedYou|OverviewGroup::Problems|OverviewGroup::Active))
            .filter_map(|s|Some((s.parent_id.clone()?,s.activity_generation?))).collect::<Vec<_>>();
        let mut changed=false;
        for (parent,generation) in active {
            if let Some(session)=sessions.iter_mut().find(|s|s.id==parent) {
                if session.activity_generation.is_none_or(|old|generation>old) {
                    session.activity_generation=Some(generation);
                    session.task_complete=Some(false);
                    changed=true;
                }
            }
        }
        if !changed {break;}
    }
    super::worktree::attach(sessions, activities);
}
fn fields(session: &mut OverviewSession, activity: &Activity, child: bool) {
    if child || matches!(session.runtime_status.as_str(), "" | "unknown" | "notLoaded") {
        session.runtime_status.clone_from(&activity.runtime);
    }
    if activity.parent.is_some() { session.parent_id.clone_from(&activity.parent); }
    session.task_complete = activity.complete;
    session.activity_generation = activity.generation.or(session.activity_generation);
    session.act_phase.clone_from(&activity.phase);
    session.phase_history.clone_from(&activity.phases);
    session.phase_source = activity.phase.as_ref().map(|_| "ACT hook".into());
    session.ticket.clone_from(&activity.ticket);
    session.question = if activity.pending.is_empty() { None }
        else { Some(activity.pending.values().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n")) };
    if session.question.is_some() && matches!(session.runtime_status.as_str(),"idle"|"done"|"waiting-parent") {session.runtime_status="waiting".into();}
    session.problem.clone_from(&activity.error);
    if !activity.project.is_empty() { session.project.clone_from(&activity.project); }
    if session.project_path.is_empty() { session.project_path.clone_from(&activity.cwd); }
    if !child {
        if let Some(title) = &activity.title { session.title.clone_from(title); }
    }
    classify(session);
}
pub fn classify(session: &mut OverviewSession) {
    session.group = if session.question.as_ref().is_some_and(|s| !s.is_empty()) || session.needs_user {
        OverviewGroup::NeedYou
    } else if session.problem.is_some() || matches!(session.runtime_status.as_str(), "error" | "failed" | "systemError" | "blocked") {
        OverviewGroup::Problems
    } else if session.manual_completion.or(session.task_complete) == Some(true) {
        OverviewGroup::Done
    } else if matches!(session.runtime_status.as_str(), "working" | "active") {
        OverviewGroup::Active
    } else { OverviewGroup::Unknown };
}

pub fn retain_completed(sessions:&mut Vec<OverviewSession>,activities:&[Activity],manual:impl Fn(&str)->Option<bool>)->bool {
    let mut added=false;
    for activity in activities {
        if activity.parent.is_some() || (activity.pane_id.is_none() && activity.core_terminal.is_none()) || manual(&activity.key).or(activity.complete)!=Some(true)
            || sessions.iter().any(|s|s.id==activity.key) {continue;}
        sessions.push(OverviewSession {
            id:activity.key.clone(),title:activity.title.clone().unwrap_or_else(||"Abgeschlossener Auftrag".into()),program:activity.provider.clone(),
            preview:String::new(),project:activity.project.clone(),project_path:activity.cwd.clone(),ticket:activity.ticket.clone(),
            act_phase:activity.phase.clone(),phase_history:activity.phases.clone(),worktree:None,phase_source:Some("ACT hook".into()),
            manual_completion:manual(&activity.key),task_complete:activity.complete,question:None,needs_user:false,problem:None,
            hidden:false,runtime_status:"closed".into(),group:OverviewGroup::Done,completion_seq:None,
            activity_generation:activity.generation,parent_id:None,source:SessionSource::Ended,
        });
        added=true;
    }
    added
}
