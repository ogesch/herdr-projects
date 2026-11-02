//! Translate native Herdr spaces and core terminals into the shared overview model.
use super::{overview::{self,OverviewSession,SessionSource},overview_events};
use crate::json::{Js,Value};
use serde_json::json;

pub fn from_app(app:&Value,terminals:&[Value])->Vec<OverviewSession> {
    let mut sessions=Vec::new();
    for space in app.arr("spaces") {
        for tab in space.arr("tabs") {
            for terminal in tab.arr("terminals") {add(terminal,None,terminals,&mut sessions);}
        }
    }
    sessions
}
fn add(node:&Value,parent:Option<&str>,core:&[Value],sessions:&mut Vec<OverviewSession>) {
    let address=node.as_str().or(node.so("terminal")).or(node.so("address")).unwrap_or("");
    if address.is_empty() {return;}
    let detail=core.iter().find(|t|t.s("address")==address);
    let provider=node.so("app").or_else(||detail.and_then(|t|t.at("status").so("app"))).unwrap_or("");
    let provider=match provider.to_lowercase().replace([' ','_','-'],"").as_str() {"codex"|"codexcli"=>"codex","claude"|"claudecode"=>"claude",_=>return};
    let runtime=node.so("status").or_else(||detail.and_then(|t|t.at("status").so("state"))).unwrap_or("unknown");
    let runtime=if parent.is_some() && matches!(runtime,"idle"|"done") {"waiting-parent"} else {runtime};
    let title=node.so("name").or_else(||detail.and_then(|t|t.so("title"))).unwrap_or(provider);
    let mut session=overview::from_herdr(&[json!({"agent":provider,"pane_id":address,"name":title,"agent_status":runtime})],&[]).remove(0);
    session.id=format!("core-terminal:{address}");
    session.parent_id=parent.map(str::to_owned);
    session.project_path=detail.and_then(|t|t.so("cwd")).unwrap_or("").into();
    let project=overview_events::context::resolve(&session.project_path,"").project;
    if !project.is_empty() {session.project=project;}
    session.needs_user=runtime=="blocked";
    session.source=SessionSource::Native{terminal:address.into()};
    overview_events::project::classify(&mut session);
    let id=session.id.clone();sessions.push(session);
    for child in node.arr("sub_agents") {add(child,Some(&id),core,sessions);}
}
