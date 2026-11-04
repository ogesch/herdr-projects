//! Find the linked git worktree a session works in, so a coordinator in the main checkout still names its feature branch.
use super::Activity;
use crate::agents::overview::OverviewSession;
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Worktree {
    pub path: String,
    pub branch: Option<String>,
}

/// The linked worktree containing `start`; the main checkout and submodules are not linked worktrees.
pub fn linked(start: &Path) -> Option<Worktree> {
    if !start.is_absolute() { return None; }
    let mut dir = start;
    loop {
        let git = dir.join(".git");
        if git.is_dir() { return None; }
        if git.is_file() {
            let text = fs::read_to_string(&git).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            let gitdir = if Path::new(target).is_absolute() { PathBuf::from(target) } else { dir.join(target) };
            if gitdir.parent()?.file_name()? != "worktrees" { return None; }
            let head = fs::read_to_string(gitdir.join("HEAD")).ok();
            let branch = head.as_deref().and_then(|h| h.trim().strip_prefix("ref: refs/heads/")).map(str::to_owned);
            return Some(Worktree { path: dir.display().to_string(), branch });
        }
        dir = dir.parent()?;
    }
}

pub fn attach(sessions: &mut [OverviewSession], activities: &[Activity]) {
    for session in sessions.iter_mut() {
        let activity = activities.iter().find(|a| a.key == session.id);
        let remembered = activity.and_then(|a| a.worktree.as_deref()).and_then(|w| linked(Path::new(w)));
        let cwd = activity.map(|a| a.cwd.as_str()).filter(|c| !c.is_empty()).unwrap_or(&session.project_path);
        session.worktree = remembered.or_else(|| linked(Path::new(cwd)));
    }
    // A coordinator stays in the main checkout; its most recently active subagent shows where the work happens.
    for _ in 0..sessions.len() {
        let mut changed = false;
        for i in 0..sessions.len() {
            if sessions[i].worktree.is_some() { continue; }
            let id = sessions[i].id.clone();
            let found = sessions.iter().filter(|c| c.parent_id.as_deref() == Some(id.as_str()) && c.worktree.is_some())
                .max_by_key(|c| c.activity_generation).and_then(|c| c.worktree.clone());
            if found.is_some() { sessions[i].worktree = found; changed = true; }
        }
        if !changed { break; }
    }
}
