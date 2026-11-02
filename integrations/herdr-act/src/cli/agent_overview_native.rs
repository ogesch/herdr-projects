//! Native app snapshots and the same durable corrections used by the terminal overview.
use crate::{agents::{codex_app_cache,overview_native,overview_events,overview_store::OverviewStore,overview_tree},err::{Error,Res},json::{Js,Value}};
use serde_json::json;
use std::process::Command;

fn read(command:&str,args:&[&str])->Res<Value> {
    let output=Command::new(command).args(args).output()?;
    if !output.status.success() {return Err(Error::new(String::from_utf8_lossy(&output.stderr)));}
    Ok(serde_json::from_slice(&output.stdout)?)
}
pub fn snapshot()->Res<Value> {
    let app=read("herdr-app",&["ls"])?;
    let listed=read("herdr-core",&["ls","--json"])?;
    let mut details=Vec::new();let mut errors=Vec::new();
    for terminal in listed.arr("terminals") {
        if !matches!(terminal.s("agent"),"codex"|"claude"|"claude-code") {continue;}
        match read("herdr-core",&["get",terminal.s("address"),"--json"]) {Ok(v)=>details.push(v),Err(e)=>errors.push(e.to_string())}
    }
    let mut sessions=overview_native::from_app(&app,&details);
    let mut codex=json!({"refreshing":false,"updatedAt":null});
    let running=!crate::store::files::lock_free(codex_app_cache::LOCK);
    match codex_app_cache::read_at(&codex_app_cache::directory(),overview_events::now_ms() as i64,running) {
        Ok(mut cached)=>{
            if cached.refresh_due {
                cached.refreshing=crate::housekeeping::spawn_loose(&["agents","refresh-codex"])>0;
                if !cached.refreshing {errors.push("Codex-Aktualisierung konnte nicht gestartet werden.".into());}
            }
            sessions.extend(crate::agents::overview::from_codex_app(&cached.threads));
            if let Some(error)=cached.error {errors.push(error);}
            codex=json!({"refreshing":cached.refreshing,"updatedAt":cached.updated_at});
        },
        Err(e)=>errors.push(e.to_string()),
    }
    let mut store=OverviewStore::default();
    if let Some(error)=store.error() {return Err(Error::new(error));}
    overview_events::enrich(&mut sessions,|id|store.completion(id))?;
    let mut changed=false;
    for session in &mut sessions {changed|=store.apply(session);session.preview.clear();}
    if changed {store.save()?;}
    sessions.sort_by(|a,b|a.project.cmp(&b.project).then_with(||a.title.cmp(&b.title)));
    let rows=overview_tree::rows(&sessions).iter().map(|r|json!({"session":r.session,"depth":r.depth,"group":r.group})).collect::<Vec<_>>();
    Ok(json!({"rows":rows,"errors":errors,"codex":codex}))
}
pub fn update(raw:&str)->Res<()> {
    let request:Value=serde_json::from_str(raw)?;
    let id=request.s("id");
    if id.is_empty() || id.len()>1024 {return Err(Error::new("Missing overview session identity"));}
    let mut store=OverviewStore::default();
    if let Some(error)=store.error() {return Err(Error::new(error));}
    let correction=store.correction(id);
    if let Some(fields)=request.get("fields").and_then(Value::as_object) {
        for(key,target)in[("title",&mut correction.title),("project",&mut correction.project),("ticket",&mut correction.ticket)] {
            if let Some(value)=fields.get(key) {*target=value.as_str().filter(|s|!s.trim().is_empty()).map(|s|s.chars().take(4000).collect());}
        }
    }
    if request.get("completion").is_some() {
        correction.manual_completion=request["completion"].as_bool();
        correction.completion_generation=request["generation"].as_u64();
    }
    if let Some(hidden)=request["hidden"].as_bool() {
        correction.hidden=hidden;correction.hidden_generation=request["generation"].as_u64();
    }
    if let Some(closed)=request["closed"].as_bool() {
        correction.closed=closed;correction.closed_generation=request["generation"].as_u64();
    }
    if let Some(phase)=request["fields"]["phase"].as_str() {
        if phase.is_empty() {store.correction(id).act_phase=None;}
        else {store.record_phase(id,phase.chars().take(200).collect(),overview_events::now_ms());}
    }
    store.save()?;Ok(())
}
