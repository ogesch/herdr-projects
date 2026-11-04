//! ACT lifecycle, explicit completion, subagent decisions, and additive hook installation.
use crate::common;
use sw::{agents::{overview::{from_herdr, OverviewGroup}, overview_events::{self as events, Activity, Event}, overview_store::OverviewStore, overview_tree}, json::json};
use std::{fs, os::unix::fs::PermissionsExt};
const AT:u64=1_800_000_000_000;

fn event(provider:&str,session:&str,name:&str,at:u64,extra:serde_json::Value)->Event {
    let mut input=json!({"session_id":session,"hook_event_name":name,"cwd":"/repos/demo","turn_id":"turn"});
    input.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    events::normalize::from_hook(provider,&input,at).unwrap().remove(0)
}
fn main_session(provider:&str,id:&str)->sw::agents::overview::OverviewSession {
    from_herdr(&[json!({"terminal_id":"terminal","pane_id":"pane","agent_status":"idle","agent_session":{"agent":provider,"source":provider,"value":id}})],&[]).remove(0)
}
#[test]
fn act_repetitions_skips_and_land_feature_are_recorded_without_finishing_the_assignment() {
    let _w=common::world();
    let mut activity=Activity::default();
    for (i,command) in ["/act-interview-flutter","/act-implement","/act-implement","$land-feature"].iter().enumerate() {
        events::fold::apply(&mut activity,&event("codex","root","UserPromptSubmit",AT+i as u64,json!({"prompt":command,"turn_id":format!("turn-{i}")})));
    }
    assert_eq!(activity.phases.iter().map(|p|p.phase.as_str()).collect::<Vec<_>>(),["Interview","Implement","Implement","Land Feature"]);
    let done_phase=Event{key:activity.key.clone(),provider:"codex".into(),session:"root".into(),at:AT+10,kind:"report".into(),command:Some("land-feature".into()),phase_end:true,..Default::default()};
    events::fold::apply(&mut activity,&done_phase);
    assert_eq!(activity.phases.last().unwrap().ended_at,Some(AT+10));
    events::fold::apply(&mut activity,&event("codex","root","Stop",AT+11,json!({})));
    assert_eq!(activity.complete,Some(false),"phase/turn end does not finish the assignment");
}
#[test]
fn only_explicit_task_completion_finishes_work_and_a_new_prompt_reopens_hidden_work() {
    let w=common::world();
    let mut activity=Activity::default();
    events::fold::apply(&mut activity,&event("codex","root","UserPromptSubmit",AT,json!({"prompt":"/act-implement"})));
    let complete=Event{key:activity.key.clone(),provider:"codex".into(),session:"root".into(),at:AT+1,kind:"report".into(),complete:Some(true),..Default::default()};
    events::fold::apply(&mut activity,&complete);
    events::fold::apply(&mut activity,&event("codex","root","Stop",AT+2,json!({})));
    let mut sessions=vec![main_session("codex","root")];
    events::project::apply(&mut sessions,&[activity.clone()]);
    assert_eq!(sessions[0].group,OverviewGroup::Done);
    let mut store=OverviewStore::at(w.base.join("corrections.json"));store.hide(&sessions[0]);store.save().unwrap();
    events::fold::apply(&mut activity,&event("codex","root","UserPromptSubmit",AT+3,json!({"prompt":"New work","turn_id":"next"})));
    events::project::apply(&mut sessions,&[activity]);
    let mut reopened=OverviewStore::at(w.base.join("corrections.json"));assert!(reopened.apply(&mut sessions[0]));
    assert!(!sessions[0].hidden);assert_eq!(sessions[0].task_complete,Some(false));
}
#[test]
fn cli_subagent_user_questions_escalate_the_family_without_changing_parent_runtime() {
    let _w=common::world();
    for provider in ["codex","claude"] {
        let mut child=Activity::default();
        events::fold::apply(&mut child,&event(provider,"root","SubagentStart",AT,json!({"agent_id":"child","agent_type":"reviewer"})));
        events::fold::apply(&mut child,&event(provider,"root","PreToolUse",AT+1,json!({"agent_id":"child","tool_name":"request_user_input","tool_use_id":"q","tool_input":{"questions":[{"question":"Welche Variante?","options":[{"label":"A"},{"label":"B"}]}]}})));
        let mut sessions=vec![main_session(provider,"root")];
        events::project::apply(&mut sessions,&[child.clone()]);
        assert_eq!(sessions.len(),2);
        assert_eq!(sessions[0].runtime_status,"idle");
        assert_eq!(sessions[1].parent_id.as_deref(),Some(sessions[0].id.as_str()));
        assert_eq!(sessions[1].question.as_deref(),Some("Welche Variante? (A / B)"));
        assert_eq!(sessions[1].source,sessions[0].source,"answer through the existing parent terminal");
        let rows=overview_tree::rows(&sessions);
        assert_eq!(rows.iter().filter(|r|r.depth==0).count(),1);
        assert_eq!((rows[0].group,rows[1].depth),(OverviewGroup::NeedYou,1));
        events::fold::apply(&mut child,&event(provider,"child","PostToolUse",AT+2,json!({"tool_name":"request_user_input","tool_use_id":"q"})));
        assert!(child.pending.is_empty());
    }
}
#[test]
fn permission_requests_keep_the_actual_command_and_resolve_by_tool_id() {
    let _w=common::world();
    let mut a=Activity::default();
    events::fold::apply(&mut a,&event("codex","root","PermissionRequest",AT,json!({"tool_name":"Bash","tool_use_id":"approval","tool_input":{"command":"git push origin feature"}})));
    assert_eq!(a.pending["approval"].text,"git push origin feature");
    events::fold::apply(&mut a,&event("codex","root","PostToolUse",AT+1,json!({"tool_name":"Bash","tool_use_id":"other"})));
    assert_eq!(a.pending.len(),1,"another tool does not answer the approval");
    events::fold::apply(&mut a,&event("codex","root","PostToolUse",AT+2,json!({"tool_name":"Bash","tool_use_id":"approval"})));
    assert!(a.pending.is_empty());
}
#[test]
fn concurrent_hook_writers_preserve_each_phase_and_private_snapshots() {
    let w=common::world();let dir=w.base.join("signals");
    let writers=(0..12).map(|i|{
        let dir=dir.clone();
        std::thread::spawn(move||{
            let e=event("codex","root","UserPromptSubmit",AT+i,json!({"prompt":"/act-implement","turn_id":format!("turn-{i}")}));
            events::record_at(&dir,&e).unwrap();
        })
    }).collect::<Vec<_>>();
    for writer in writers {writer.join().unwrap();}
    let activities=events::load_at(&dir).unwrap();
    assert_eq!(activities.len(),1);assert_eq!(activities[0].phases.len(),12);
    assert_eq!(activities[0].last_seen,AT+11);
    let file=fs::read_dir(&dir).unwrap().map(|e|e.unwrap().path()).find(|p|p.extension().is_some_and(|x|x=="json")).unwrap();
    assert_eq!(fs::metadata(file).unwrap().permissions().mode()&0o777,0o600);
}
#[test]
fn observer_installation_keeps_existing_hooks_permissions_and_trust_and_is_idempotent() {
    let w=common::world();let codex=w.base.join("codex");let claude=w.base.join("claude");let data=w.base.join("observer");
    fs::create_dir_all(&codex).unwrap();fs::create_dir_all(&claude).unwrap();
    let original=json!({"env":{"KEEP":"yes"},"permissions":{"deny":["Bash(rm *)"]},"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"existing-guard"}]}]},"trust":{"keep":"exact"}});
    for p in [codex.join("hooks.json"),claude.join("settings.json")] {fs::write(p,original.to_string()).unwrap();}
    let binary=w.base.join("binary");fs::write(&binary,"reviewed observer").unwrap();
    let changed=events::install::install_at(&codex,&claude,&data,&binary).unwrap();assert_eq!(changed.len(),2);
    let installed:serde_json::Value=serde_json::from_slice(&fs::read(codex.join("hooks.json")).unwrap()).unwrap();
    assert_eq!(installed["hooks"]["PreToolUse"][0],original["hooks"]["PreToolUse"][0]);
    for key in ["env","permissions","trust"] {assert_eq!(installed[key],original[key]);}
    assert_eq!(installed["hooks"]["PermissionRequest"].as_array().unwrap().len(),1);
    assert!(events::install::install_at(&codex,&claude,&data,&binary).unwrap().is_empty());
    fs::write(&binary,"updated observer").unwrap();
    events::install::install_at(&codex,&claude,&data,&binary).unwrap();
    let updated:serde_json::Value=serde_json::from_slice(&fs::read(codex.join("hooks.json")).unwrap()).unwrap();
    assert_ne!(installed["hooks"]["Stop"][0]["hooks"][0]["command"],updated["hooks"]["Stop"][0]["hooks"][0]["command"],"new code requires new Codex hook trust");
}
#[test]
fn context_uses_the_configured_local_backend_and_never_reads_outside_it() {
    let w=common::world();fs::create_dir_all(w.root.join(".act")).unwrap();fs::create_dir_all(w.root.join("specs")).unwrap();
    fs::write(w.root.join(".act/config.yaml"),"workflow:\n  backend: local\nlocal:\n  path: specs\n").unwrap();
    fs::write(w.root.join("specs/release.md"),"# Release the journal\n").unwrap();
    fs::write(w.root.join("private.md"),"# Private\n").unwrap();
    let cwd=w.root.display().to_string();
    assert!(events::context::resolve(&cwd,"/act-implement specs/release.md").ticket.unwrap().contains("Release the journal"));
    assert!(events::context::resolve(&cwd,"/act-implement private.md #42").ticket.is_none());
    fs::write(w.root.join(".act/config.yaml"),"workflow:\n  backend: github\n").unwrap();
    assert!(events::context::resolve(&cwd,"/act-implement #42").ticket.unwrap().ends_with("#42"));
}

#[test]
fn invalid_hook_settings_are_rejected_before_changing_either_existing_configuration() {
    let w=common::world();let codex=w.base.join("codex");let claude=w.base.join("claude");
    fs::create_dir_all(&codex).unwrap();fs::create_dir_all(&claude).unwrap();
    let original=json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"guard"}]}]}});
    fs::write(codex.join("hooks.json"),original.to_string()).unwrap();
    fs::write(claude.join("settings.json"),"{invalid").unwrap();
    let binary=w.base.join("binary");fs::write(&binary,"observer").unwrap();
    assert!(events::install::install_at(&codex,&claude,&w.base.join("data"),&binary).is_err());
    let after:serde_json::Value=serde_json::from_slice(&fs::read(codex.join("hooks.json")).unwrap()).unwrap();
    assert_eq!(after,original);
}

#[test]
fn async_questions_survive_the_immediate_tool_return_and_clear_on_user_reply() {
    let _w=common::world();let mut a=Activity::default();
    events::fold::apply(&mut a,&event("codex","root","PreToolUse",AT,json!({"tool_name":"functions.request_user_input_async","tool_use_id":"q","tool_input":{"questions":[{"title":"Which project?","options":["Journal","Catalog"]}]}})));
    events::fold::apply(&mut a,&event("codex","root","PostToolUse",AT+1,json!({"tool_name":"functions.request_user_input_async","tool_use_id":"q"})));
    events::fold::apply(&mut a,&event("codex","root","Stop",AT+2,json!({})));
    assert_eq!(a.pending["q"].text,"Which project? (Journal / Catalog)");
    events::fold::apply(&mut a,&event("codex","root","UserPromptSubmit",AT+3,json!({"prompt":"Journal"})));
    assert!(a.pending.is_empty());
}
#[test]
fn corrupt_snapshots_and_corrections_are_not_overwritten_by_later_reports() {
    let w=common::world();let dir=w.base.join("snapshots");
    let e=event("codex","root","UserPromptSubmit",AT,json!({"prompt":"/act-interview"}));
    events::record_at(&dir,&e).unwrap();
    let snapshot=fs::read_dir(&dir).unwrap().map(|e|e.unwrap().path()).find(|p|p.extension().is_some_and(|x|x=="json")).unwrap();
    fs::write(&snapshot,"{damaged").unwrap();
    assert!(events::record_at(&dir,&e).is_err());
    assert_eq!(fs::read_to_string(snapshot).unwrap(),"{damaged");
    let corrections=w.base.join("corrections.json");fs::write(&corrections,"{damaged").unwrap();
    let mut store=OverviewStore::at(&corrections);store.correction("root").title=Some("new".into());
    assert!(store.save().is_err());assert_eq!(fs::read_to_string(corrections).unwrap(),"{damaged");
}
#[test]
fn skill_loading_reports_entry_and_does_not_pretend_the_phase_already_finished() {
    let _w=common::world();let mut a=Activity::default();
    events::fold::apply(&mut a,&event("claude","root","PreToolUse",AT,json!({"tool_name":"Skill","tool_use_id":"s","tool_input":{"skill":"act:act-create-spec-flutter"}})));
    events::fold::apply(&mut a,&event("claude","root","PostToolUse",AT+1,json!({"tool_name":"Skill","tool_use_id":"s"})));
    assert_eq!(a.phases.len(),1);assert_eq!(a.phase.as_deref(),Some("Spec erstellen"));assert_eq!(a.phases[0].ended_at,None);
}
#[test]
fn codex_subagents_enter_the_act_command_whose_skill_file_they_read() {
    let _w=common::world();
    let read=|id:&str,at|event("codex","child","PreToolUse",at,json!({"tool_name":"exec_command","tool_use_id":id,
        "tool_input":{"cmd":"sed -n '1,200p' ~/.agents/skills/act-implement-flutter/SKILL.md ~/.agents/skills/act-flutter-development/SKILL.md"}}));
    let mut child=Activity{parent:Some("codex-thread:root".into()),..Activity::default()};
    events::fold::apply(&mut child,&read("a",AT));events::fold::apply(&mut child,&read("b",AT+1));
    assert_eq!(child.phases.len(),1);assert_eq!(child.phases[0].command.as_deref(),Some("act-implement-flutter"));
    assert_eq!(child.phase.as_deref(),Some("Implement"));
    let mut root=Activity::default();
    events::fold::apply(&mut root,&read("a",AT));
    assert!(root.phases.is_empty() && root.phase.is_none());
}
#[test]
fn coordinators_in_the_main_checkout_show_the_worktree_their_subagents_work_in() {
    let w=common::world();let repo=w.base.join("repo");let tree=repo.join("worktrees/feat/139-sections");
    fs::create_dir_all(repo.join(".git/worktrees/139-sections")).unwrap();fs::create_dir_all(tree.join("lib")).unwrap();
    fs::write(repo.join(".git/worktrees/139-sections/HEAD"),"ref: refs/heads/feat/139-sections\n").unwrap();
    fs::write(tree.join(".git"),format!("gitdir: {}\n",repo.join(".git/worktrees/139-sections").display())).unwrap();
    assert_eq!(events::worktree::linked(&repo),None);
    let found=events::worktree::linked(&tree.join("lib")).unwrap();
    assert_eq!(found.path,tree.display().to_string());assert_eq!(found.branch.as_deref(),Some("feat/139-sections"));
    let mut root=main_session("codex","root");root.project_path=repo.display().to_string();
    let mut sessions=vec![root];
    let activity=|key:&str,cwd:&std::path::Path,parent:Option<&str>|Activity{key:key.into(),provider:"codex".into(),cwd:cwd.display().to_string(),
        parent:parent.map(str::to_owned),generation:Some(AT),..Activity::default()};
    let root_key=sessions[0].id.clone();
    events::project::apply(&mut sessions,&[activity(&root_key,&repo,None),activity("codex-thread:child",&tree,Some(&root_key))]);
    let mut codex=Activity{key:"codex-thread:late".into(),parent:Some(root_key.clone()),cwd:repo.display().to_string(),..Activity::default()};
    events::fold::apply(&mut codex,&event("codex","late","PreToolUse",AT,json!({"cwd":repo,"tool_name":"exec_command","tool_input":{"cmd":"flutter test","workdir":tree.join("lib")}})));
    events::fold::apply(&mut codex,&event("codex","late","Stop",AT+1,json!({"cwd":repo})));
    assert_eq!(codex.worktree.as_deref(),Some(tree.to_str().unwrap()));
    let child=sessions.iter().find(|s|s.id=="codex-thread:child").unwrap();
    assert_eq!(child.worktree.as_ref(),Some(&found));
    assert_eq!(sessions.iter().find(|s|s.id==root_key).unwrap().worktree.as_ref(),Some(&found));
}
#[test]
fn real_programs_agent_overview_hook_outputs_context_but_never_permission_decisions() {
    use std::{process::{Command,Stdio},io::Write};
    let w=common::world();let data=w.base.join("wire-events");
    for name in ["SessionStart","PermissionRequest","Stop"] {
        let mut child=Command::new(env!("CARGO_BIN_EXE_herdr-projects")).args(["agents","hook","codex"])
            .current_dir(&w.root).env("HERDR_PROJECTS_DATA_DIR",&data)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let payload=json!({"session_id":"wire-fixture","originator":"codex_work_desktop","hook_event_name":name,"cwd":w.root,"tool_name":"Bash","tool_use_id":"approval","tool_input":{"command":"git push"}});
        child.stdin.take().unwrap().write_all(payload.to_string().as_bytes()).unwrap();
        let output=child.wait_with_output().unwrap();assert!(output.status.success(),"{:?}",output);
        let answer:serde_json::Value=serde_json::from_slice(&output.stdout).unwrap();
        assert!(answer.get("decision").is_none() && answer.get("continue").is_none());
        if name=="SessionStart" {assert!(answer["hookSpecificOutput"]["additionalContext"].as_str().unwrap().contains("--state done"));}
        else {assert_eq!(answer,json!({}));}
    }
    let activity=events::load_at(&data.join("agent-overview/sessions")).unwrap().remove(0);
    assert_eq!(activity.runtime,"idle");assert_ne!(activity.complete,Some(true));assert!(activity.pending.is_empty());
}

#[test]
fn late_hook_writers_keep_history_order_and_do_not_replace_the_current_phase_or_title() {
    let _w=common::world();let mut a=Activity::default();
    events::fold::apply(&mut a,&event("codex","root","UserPromptSubmit",AT+2,json!({"prompt":"/act-implement","turn_id":"new"})));
    events::fold::apply(&mut a,&event("codex","root","UserPromptSubmit",AT,json!({"prompt":"/act-interview","turn_id":"old"})));
    assert_eq!(a.phase.as_deref(),Some("Implement"));assert_eq!(a.title.as_deref(),Some("/act-implement"));
    assert_eq!(a.phases.iter().map(|p|p.phase.as_str()).collect::<Vec<_>>(),["Interview","Implement"]);
}
#[test]
fn cli_capture_stays_inside_herdr_and_desktop_identity_comes_from_session_metadata() {
    let w=common::world();let codex=w.base.join("codex");let sessions=codex.join("sessions");
    fs::create_dir_all(&sessions).unwrap();let transcript=sessions.join("session.jsonl");
    let input=json!({"session_id":"root","transcript_path":transcript});
    fs::write(&transcript,json!({"type":"session_meta","payload":{"originator":"codex_work_desktop"}}).to_string()).unwrap();
    assert!(events::scope::allows("codex",&input,false,&codex,&w.base.join("signals")));
    fs::write(&transcript,json!({"type":"session_meta","payload":{"originator":"codex-tui"}}).to_string()).unwrap();
    assert!(!events::scope::allows("codex",&input,false,&codex,&w.base.join("signals")));
    assert!(!events::scope::allows("claude",&input,false,&codex,&w.base.join("signals")));
    assert!(events::scope::allows("claude",&input,true,&codex,&w.base.join("signals")));
}

#[test]
fn hooks_join_a_cli_pane_without_native_thread_identity_and_keep_its_saved_corrections() {
    let w=common::world();
    let mut sessions=from_herdr(&[json!({"agent":"Codex","terminal_id":"terminal","pane_id":"w1:p2","agent_status":"idle"})],&[]);
    let mut activity=Activity::default();
    let mut start=event("codex","root","SessionStart",AT,json!({}));start.pane_id=Some("w1:p2".into());
    events::fold::apply(&mut activity,&start);
    sessions[0].project="feature-branch".into();activity.project="demo".into();
    let mut store=OverviewStore::at(w.base.join("corrections.json"));
    store.correction("herdr-pane:w1:p2").title=Some("Saved work title".into());
    events::project::apply(&mut sessions,&[activity]);
    assert_eq!(sessions[0].id,"codex-thread:root");assert_eq!(sessions[0].program,"codex");
    assert_eq!(sessions[0].project,"demo","observed repository context replaces native branch labels");
    assert!(store.apply(&mut sessions[0]));assert_eq!(sessions[0].title,"Saved work title");
    store.save().unwrap();
    let mut reloaded=OverviewStore::at(w.base.join("corrections.json"));reloaded.apply(&mut sessions[0]);
    assert_eq!(sessions[0].title,"Saved work title");
}

#[test]
fn completed_cli_assignments_survive_agent_exit_and_new_observed_work_reopens_them() {
    let w=common::world();
    let mut activity=Activity{key:"codex-thread:root".into(),provider:"codex".into(),session:"root".into(),
        pane_id:Some("pane".into()),project:"demo".into(),title:Some("Finished assignment".into()),complete:Some(true),generation:Some(AT),runtime:"closed".into(),..Default::default()};
    let mut sessions=vec![];
    assert!(events::project::retain_completed(&mut sessions,&[activity.clone()],|_|None));
    events::project::apply(&mut sessions,&[activity.clone()]);
    assert_eq!(sessions[0].group,OverviewGroup::Done);
    assert!(matches!(sessions[0].source,sw::agents::overview::SessionSource::Ended));
    let mut store=OverviewStore::at(w.base.join("corrections.json"));store.hide(&sessions[0]);store.save().unwrap();
    events::fold::apply(&mut activity,&event("codex","root","UserPromptSubmit",AT+1,json!({"prompt":"New assignment","turn_id":"next"})));
    let mut resumed=vec![main_session("codex","root")];events::project::apply(&mut resumed,&[activity]);
    assert!(store.apply(&mut resumed[0]));assert!(!resumed[0].hidden);assert_ne!(resumed[0].group,OverviewGroup::Done);
}
