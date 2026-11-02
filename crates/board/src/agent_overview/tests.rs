//! Render four parent agents and subagents at compact and wide terminal sizes without live integrations.
use super::*;
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn four_families_render_in_compact_and_wide_views_and_keep_selection() {
    let agents=(0..4).map(|i|serde_json::json!({
        "terminal_id":format!("t{i}"),"pane_id":format!("p{i}"),"name":format!("Main {i}"),"agent_status":"idle",
        "agent_session":{"agent":"codex","source":"codex","value":format!("root{i}")}
    })).collect::<Vec<_>>();
    let mut sessions=overview::from_herdr(&agents,&[]);
    let activities=(0..4).map(|i|overview_events::Activity {
        key:format!("codex-thread:child{i}"),provider:"codex".into(),session:format!("child{i}"),
        parent:Some(format!("codex-thread:root{i}")),title:Some(format!("Review {i}")),runtime:"working".into(),
        ..Default::default()
    }).collect::<Vec<_>>();
    overview_events::project::apply(&mut sessions,&activities);
    let(tx,rx)=mpsc::channel();
    let mut app=App{
        herdr:Herdr::new(None),herdr_sessions:vec![],codex_sessions:vec![],sessions,
        store:OverviewStore::at(std::env::temp_dir().join(format!("overview-render-unused-{}",std::process::id()))),
        selected:0,project:None,show_hidden:false,collapsed:BTreeSet::new(),collapsed_groups:BTreeSet::new(),expanded_history:BTreeSet::new(),detail_scroll:0,
        terminal_text:String::new(),error:None,codex_error:None,codex_rx:rx,codex_tx:tx,codex_inflight:false,
        next_herdr:Instant::now(),next_codex:Instant::now(),attached_pane:None,
    };
    for(width,height)in[(80,32),(120,36),(180,50)] {
        let mut terminal=Terminal::new(TestBackend::new(width,height)).unwrap();
        terminal.draw(|frame|app.draw(frame)).unwrap();
        let text=terminal.backend().buffer().content.iter().map(|c|c.symbol()).collect::<String>();
        assert!(text.contains("Main 0")&&text.contains("Review 0"),"{width}: {text}");
        if width>=120 {assert!(text.contains("Main 3")&&text.contains("Review 3"),"{width}: {text}");}
        assert!(text.contains("Work: Unknown"),"idle parents are not completed assignments");
        app.selected=app.sessions.iter().position(|s|s.id=="codex-thread:child3").unwrap();
        terminal.draw(|frame|app.draw(frame)).unwrap();
        let text=terminal.backend().buffer().content.iter().map(|c|c.symbol()).collect::<String>();
        assert!(text.contains("Review 3"),"selection scrolls into the compact viewport");
        app.selected=0;
    }
    app.sessions[0].phase_history=(0..12).map(|i|serde_json::from_value(serde_json::json!({
        "phase":"Implement","recorded_at":i,"command":format!("command-{i}"),"source":"ACT hook"
    })).unwrap()).collect();
    let mut terminal=Terminal::new(TestBackend::new(180,50)).unwrap();
    terminal.draw(|frame|app.draw(frame)).unwrap();
    let text=terminal.backend().buffer().content.iter().map(|c|c.symbol()).collect::<String>();
    assert!(text.contains("ACT-Verlauf (12)")&&!text.contains("command-0"));
    app.expanded_history.insert(app.sessions[0].id.clone());
    app.detail_scroll=8;
    terminal.draw(|frame|app.draw(frame)).unwrap();
    let text=terminal.backend().buffer().content.iter().map(|c|c.symbol()).collect::<String>();
    assert!(text.contains("command-0")&&!text.contains("command-11"),"the full history can be expanded and scrolled");
    app.collapsed.insert("codex-thread:root0".into());
    app.sessions[0].manual_completion=Some(true);
    app.toggle_hidden();
    assert!(app.error.is_some() && !app.sessions[0].hidden,"an active child prevents hiding its family");
    app.collapsed.insert("codex-thread:root0".into());
    assert_eq!(app.visible().len(),7);
    app.collapsed_groups.insert(group_order(OverviewGroup::Active));
    assert!(app.visible().is_empty());
}
