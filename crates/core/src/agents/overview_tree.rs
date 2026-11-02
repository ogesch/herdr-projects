//! Order main agents with their descendants, escalating attention without changing the parent's runtime.
use super::overview::{OverviewGroup, OverviewSession};
use std::collections::BTreeSet;

pub struct Row<'a> { pub session: &'a OverviewSession, pub depth: usize, pub group: OverviewGroup }
pub fn rank(group: OverviewGroup) -> u8 {
    match group { OverviewGroup::NeedYou=>0, OverviewGroup::Problems=>1, OverviewGroup::Active=>2, OverviewGroup::Done=>3, OverviewGroup::Unknown=>4 }
}
pub fn rows(sessions: &[OverviewSession]) -> Vec<Row<'_>> {
    let ids = sessions.iter().map(|s| s.id.as_str()).collect::<BTreeSet<_>>();
    let mut roots = sessions.iter().filter(|s| s.parent_id.as_deref().is_none_or(|p| !ids.contains(p))).collect::<Vec<_>>();
    let remaining=sessions.iter().filter(|s| !roots.iter().any(|r| r.id == s.id)).collect::<Vec<_>>();
    roots.extend(remaining);
    let mut seen = BTreeSet::new();
    let mut families = Vec::new();
    for root in roots {
        if seen.contains(root.id.as_str()) { continue; }
        let mut family = Vec::new();
        visit(root, sessions, 0, &mut seen, &mut family);
        let mut group = root.group;
        for (session, _) in &family {
            if matches!(session.group, OverviewGroup::NeedYou | OverviewGroup::Problems | OverviewGroup::Active)
                && rank(session.group) < rank(group) { group = session.group; }
        }
        families.push((group, family));
    }
    families.sort_by(|(ga,a),(gb,b)| rank(*ga).cmp(&rank(*gb))
        .then_with(|| a[0].0.project.cmp(&b[0].0.project)).then_with(|| a[0].0.title.cmp(&b[0].0.title)));
    families.into_iter().flat_map(|(group, family)| family.into_iter().map(move |(session,depth)| Row {session,depth,group})).collect()
}
fn visit<'a>(session: &'a OverviewSession, sessions: &'a [OverviewSession], depth: usize, seen: &mut BTreeSet<&'a str>, rows: &mut Vec<(&'a OverviewSession,usize)>) {
    if !seen.insert(&session.id) { return; }
    rows.push((session,depth));
    let mut children = sessions.iter().filter(|s| s.parent_id.as_deref() == Some(&session.id)).collect::<Vec<_>>();
    children.sort_by(|a,b| a.title.cmp(&b.title));
    for child in children { visit(child, sessions, depth+1, seen, rows); }
}
