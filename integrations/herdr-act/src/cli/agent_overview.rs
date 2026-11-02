//! Agent Overview's local observer hooks, ACT reports, and live source diagnostics.
use super::flags::Args;
use crate::{agents::{codex_app, overview, overview_events::{self as events, Event}}, err::{Error, Res}, json::Js};
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Read};

pub fn run(args: &Args) -> Res<i32> {
    let allowed = ["source","session","phase","phase-state","ticket","state","question","help"];
    if let Some(key) = args.typed.keys().find(|key| !allowed.contains(&key.as_str())) {
        crate::refuse!("agents does not know --{key}");
    }
    if args.pos.get(1).is_none_or(|s|s!="report") {
        if let Some(key)=args.typed.keys().find(|key|key.as_str()!="help") {crate::refuse!("This agents command does not take --{key}");}
    }
    if args.flags.b("help") {print!("{}",super::help::HELP);return Ok(0);}
    match args.pos.get(1).map(String::as_str).unwrap_or("") {
        "native-snapshot" => {println!("{}",super::agent_overview_native::snapshot()?);Ok(0)},
        "conversation" => {
            let key=args.pos.get(2).ok_or("conversation needs a session identity")?;
            println!("{}",crate::agents::overview_conversation::read(key)?);
            Ok(0)
        },
        "refresh-codex" => {
            let cache = crate::agents::codex_app_cache::directory();
            crate::agents::codex_app_cache::refresh_at(&cache, || events::now_ms() as i64, codex_app::list_threads)?;
            Ok(0)
        },
        "native-update" => {super::agent_overview_native::update(args.pos.get(2).ok_or("native-update needs JSON")?)?;Ok(0)},
        "" => Ok(crate::agent_overview::open_overview()),
        "hook" => {
            let provider = args.pos.get(2).map(String::as_str).unwrap_or("");
            if let Err(error) = hook(provider) {
                eprintln!("Herdr overview observer: {error}");
                println!("{{}}");
            }
            Ok(0)
        }
        "install-hooks" => {
            let paths = events::install::install()?;
            for path in paths { println!("Installed observer: {}", path.display()); }
            println!("Codex: review the new observer definitions with /hooks; existing trust is preserved.");
            Ok(0)
        }
        "report" => { report(args)?; Ok(0) }
        "check" => {
            let threads = codex_app::list_threads()?;
            let mut sessions = overview::from_codex_app(&threads);
            let activities = events::load_at(&events::directory())?;
            events::project::apply(&mut sessions,&activities);
            let mut states = BTreeMap::<String, usize>::new();
            for session in &sessions { *states.entry(session.runtime_status.clone()).or_default() += 1; }
            let mut all_states=BTreeMap::<String,usize>::new();
            let mut loaded_origins=BTreeMap::<String,usize>::new();
            for thread in &threads {
                let state=thread["status"]["type"].as_str().unwrap_or("unknown");
                *all_states.entry(state.into()).or_default()+=1;
                if state!="notLoaded" {*loaded_origins.entry(thread["originator"].as_str().unwrap_or("unknown").into()).or_default()+=1;}
            }
            let mut phases=BTreeMap::<String,usize>::new();
            for session in &sessions {if let Some(phase)=&session.act_phase {*phases.entry(phase.clone()).or_default()+=1;}}
            println!("{}", json!({"codexDesktopThreads":sessions.len(),"runtimeStates":states,"actPhases":phases,"allRuntimeStates":all_states,"loadedOriginators":loaded_origins,"hookSessions":activities.len()}));
            Ok(0)
        }
        other => crate::refuse!("Unknown agents command: {other}"),
    }
}
fn hook(provider: &str) -> Res<()> {
    let mut raw = String::new();
    std::io::stdin().take(1_048_577).read_to_string(&mut raw)?;
    if raw.len() > 1_048_576 { return Err(Error::new("Hook payload exceeds 1 MiB")); }
    let input: Value = serde_json::from_str(&raw)?;
    let native=events::scope::native_terminal();
    if !events::scope::allows(provider,&input,crate::util::env_val("HERDR_ENV").as_deref()==Some("1") || native.is_some(),&crate::agents::homes::codex_home(),&events::directory()) {println!("{{}}");return Ok(());}
    let mut normalized = events::normalize::from_hook(provider, &input, events::now_ms())?;
    for event in &mut normalized {
        event.pane_id=crate::util::env_val("HERDR_PANE_ID");event.core_terminal.clone_from(&native);
        if event.parent.is_none() {event.parent=native.as_deref().and_then(events::scope::native_parent);}
    }
    for event in &normalized { events::record(event)?; }
    let name = input["hook_event_name"].as_str().unwrap_or("");
    if name == "SessionStart" || (name == "UserPromptSubmit" && normalized.iter().any(|e| e.phase.is_some())) {
        let session = normalized.first().map(|e|e.session.as_str()).ok_or("Missing session_id")?;
        println!("{}", json!({"hookSpecificOutput":{"hookEventName":name,"additionalContext":events::context::instructions(provider,session)?}}));
    } else { println!("{{}}"); }
    Ok(())
}
fn report(args: &Args) -> Res<()> {
    let f = &args.flags;
    let source = f.s("source");
    let session = f.s("session");
    if !matches!(source, "codex" | "claude") || session.is_empty() { crate::refuse!("report needs --source codex|claude and --session ID"); }
    let command = f.so("phase").map(str::to_owned);
    let phase = command.as_deref().map(|p| events::normalize::phase(p).ok_or_else(|| Error::refused(format!("Unknown ACT command: {p}")))).transpose()?.map(str::to_owned);
    let state = f.s("state");
    if !matches!(state, "" | "done" | "open") { crate::refuse!("state must be done or open"); }
    let phase_state = f.s("phase-state");
    if !matches!(phase_state, "" | "start" | "end") || (phase_state == "end" && command.is_none()) {
        crate::refuse!("phase-state must be start or end, with --phase COMMAND");
    }
    if state == "done" && f.so("question").is_some() { crate::refuse!("A task waiting on a question cannot be reported done"); }
    if command.is_none() && state.is_empty() && f.so("question").is_none() && f.so("ticket").is_none() {
        crate::refuse!("report needs a phase, ticket, question, or assignment state");
    }
    let event = Event {
        key: events::session_key(source, session), provider: source.into(), session: session.into(),
        cwd: std::env::current_dir()?.display().to_string(), at: events::now_ms(), kind: "report".into(), via: "ACT report".into(),
        pane_id:crate::util::env_val("HERDR_PANE_ID"),
        core_terminal:events::scope::native_terminal(),
        parent:events::scope::native_terminal().as_deref().and_then(events::scope::native_parent),
        command, phase, phase_end: phase_state == "end", ticket: f.so("ticket").map(str::to_owned),
        text: f.so("question").map(|s| s.chars().take(4000).collect()),
        complete: if state.is_empty() { None } else { Some(state == "done") }, ..Default::default()
    };
    events::record(&event)?;
    Ok(())
}
