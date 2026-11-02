//! Agent overview grouping and correction persistence tests.
use sw::{
    agents::{
        overview::{from_codex_app, from_herdr, OverviewGroup, SessionSource},
        overview_store::OverviewStore,
    },
    json::json,
};

fn test_file() -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("herdr-agent-overview-{}-{nonce}.json", std::process::id()))
}

#[test]
fn herdr_session_status_completion_and_project_stay_separate() {
    let agents = vec![
        json!({"terminal_id":"term1","pane_id":"p1","workspace_id":"w1","agent":"claude","agent_status":"idle","state_change_seq":4}),
        json!({"terminal_id":"term2","pane_id":"p2","workspace_id":"w1","agent":"codex","agent_status":"idle","completion_seq":2,"state_change_seq":6,
            "agent_session":{"source":"codex","agent":"codex","kind":"thread","value":"thread2"}}),
    ];
    let workspaces = vec![
        json!({"workspace_id":"w1","label":"old label","worktree":{"repo_name":"cheekily","repo_root":"/repo","checkout_path":"/repo/worktrees/feature"}}),
    ];
    let sessions = from_herdr(&agents, &workspaces);

    assert_eq!(sessions[0].group, OverviewGroup::Unknown);
    assert_eq!(sessions[0].completion_seq, None);
    assert_eq!(sessions[0].project, "cheekily");
    assert_eq!(sessions[0].project_path, "/repo/worktrees/feature");
    assert_eq!(sessions[1].group, OverviewGroup::Unknown);
    assert_eq!(sessions[1].runtime_status, "idle");
    assert_eq!(sessions[1].id, "codex-thread:thread2");
    assert!(matches!(&sessions[1].source, SessionSource::Herdr { target, .. } if target=="p2"));
}

#[test]
fn codex_app_threads_use_runtime_flags_and_only_desktop_origins() {
    let threads = vec![
        json!({"id":"a","originator":"Codex Desktop","status":{"type":"active","activeFlags":["waitingOnApproval"]},"parentThreadId":"root","cwd":"/repo/app"}),
        json!({"id":"b","originator":"codex_work_desktop","status":{"type":"systemError"}}),
        json!({"id":"c","originator":"codex-tui","status":{"type":"active"}}),
    ];
    let sessions = from_codex_app(&threads);

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].group, OverviewGroup::NeedYou);
    assert_eq!(sessions[0].parent_id.as_deref(), Some("codex-thread:root"));
    assert_eq!(sessions[0].project, "app");
    assert_eq!(sessions[1].group, OverviewGroup::Problems);
    assert_eq!(sessions[1].completion_seq, None);
}

#[test]
fn corrections_and_repeated_phase_history_survive_reload() {
    let file = test_file();
    let mut store = OverviewStore::at(&file);
    store.record_phase("codex-thread:a", "implement".into(), 10);
    store.record_phase("codex-thread:a", "implement".into(), 20);
    {
        let correction = store.correction("codex-thread:a");
        correction.title = Some("Release fix".into());
        correction.project = Some("cheekily".into());
        correction.ticket = Some("ISSUE-8".into());
        correction.manual_completion = Some(true);
    }
    store.save().unwrap();

    let mut reloaded = OverviewStore::at(&file);
    let mut session = from_codex_app(&[json!({
        "id":"a","originator":"Codex Desktop","status":{"type":"idle"},"cwd":"/repo"
    })]).remove(0);
    reloaded.apply(&mut session);
    assert_eq!(session.title, "Release fix");
    assert_eq!(session.project, "cheekily");
    assert_eq!(session.ticket.as_deref(), Some("ISSUE-8"));
    assert_eq!(session.act_phase.as_deref(), Some("implement"));
    assert_eq!(session.phase_history.iter().map(|p| p.recorded_at).collect::<Vec<_>>(), [10_000, 20_000]);
    assert_eq!(session.group, OverviewGroup::Done);

    std::fs::remove_file(file).unwrap();
}

#[test]
fn hidden_completed_session_reappears_when_new_work_is_seen() {
    let file = test_file();
    let mut store = OverviewStore::at(&file);
    let mut session = from_herdr(
        &[json!({"terminal_id":"term","pane_id":"pane","workspace_id":"w","agent_status":"working","state_change_seq":9})],
        &[],
    ).remove(0);
    {
        let correction = store.correction(&session.id);
        correction.manual_completion = Some(true);
        correction.completion_generation = Some(8);
        correction.hidden = true;
        correction.hidden_generation = Some(8);
    }
    store.apply(&mut session);
    assert!(!session.hidden);
    assert_eq!(session.manual_completion, None);
    assert_eq!(session.group, OverviewGroup::Active);

    store.save().unwrap();
    std::fs::remove_file(file).unwrap();
}
