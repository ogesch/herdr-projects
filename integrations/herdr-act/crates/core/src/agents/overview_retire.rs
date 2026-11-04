//! Close subagents the coordinator no longer uses: superseded rounds, finished stages and children of ended parents.
//!
//! Derived on every snapshot and never stored, so a child that starts new work shows up again on its own.
//! A child with an open question, a problem or running work is never closed.
use super::overview::OverviewSession;
use std::collections::{BTreeSet, HashMap};

fn busy(s: &OverviewSession) -> bool {
    s.question.as_ref().is_some_and(|q| !q.is_empty()) || s.needs_user || s.problem.is_some()
        || matches!(s.runtime_status.as_str(), "working" | "active" | "waiting" | "blocked")
}
fn ended(s: &OverviewSession) -> bool {
    s.runtime_status == "closed" || s.manual_completion.or(s.task_complete) == Some(true)
}
/// "journal review 140 r3" → ("journal review 140", 3); a title without a round is round 1.
fn round(title: &str) -> (String, u32) {
    let lower = title.trim().to_lowercase();
    if let Some((base, n)) = lower.rsplit_once(|c: char| c == ' ' || c == '_' || c == '-') {
        if let Some(n) = n.strip_prefix('r').and_then(|n| n.parse::<u32>().ok()) {
            return (base.trim_end_matches([' ', '_', '-']).to_owned(), n);
        }
    }
    (lower, 1)
}
/// A later round of the same task under the same parent replaces this one.
fn superseded(s: &OverviewSession, siblings: &[&OverviewSession]) -> bool {
    let (base, n) = round(&s.title);
    siblings.iter().any(|o| o.id != s.id && { let (b, m) = round(&o.title); b == base && m > n })
}
/// The parent moved on to another ACT stage after this child's last activity.
fn stage_passed(s: &OverviewSession, parent: &OverviewSession) -> bool {
    let Some(last) = s.activity_generation else { return false };
    let mut phases = parent.phase_history.iter().collect::<Vec<_>>();
    phases.sort_by_key(|p| p.recorded_at);
    let Some(during) = phases.iter().rev().find(|p| p.recorded_at <= last) else { return false };
    phases.iter().any(|p| p.recorded_at > last && p.phase != during.phase)
}
pub fn retire(sessions: &mut [OverviewSession]) {
    let index = sessions.iter().enumerate().map(|(i, s)| (s.id.clone(), i)).collect::<HashMap<_, _>>();
    let mut retired = BTreeSet::new();
    loop {
        let mut changed = false;
        for s in sessions.iter() {
            let Some(parent) = s.parent_id.as_ref().and_then(|p| index.get(p)).map(|&i| &sessions[i]) else { continue };
            if retired.contains(&s.id) || busy(s) { continue; }
            let siblings = sessions.iter().filter(|o| o.parent_id == s.parent_id).collect::<Vec<_>>();
            if ended(parent) || retired.contains(&parent.id) || superseded(s, &siblings) || stage_passed(s, parent) {
                retired.insert(s.id.clone());
                changed = true;
            }
        }
        if !changed { break; }
    }
    for s in sessions.iter_mut().filter(|s| retired.contains(&s.id)) {
        s.runtime_status = "closed".into();
        s.hidden = true;
        super::overview_events::project::classify(s);
    }
}
