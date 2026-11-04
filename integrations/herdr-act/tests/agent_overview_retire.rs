//! Unused subagents leave the overview: superseded rounds, passed stages and children of ended parents.
use crate::common;
use sw::{agents::{overview::{self,OverviewSession,PhaseRecord},overview_retire},json::json};

fn session(id:&str,title:&str,parent:Option<&str>,runtime:&str,at:u64)->OverviewSession {
    let mut s=overview::from_herdr(&[json!({"agent":"codex","pane_id":id,"name":title,"agent_status":runtime})],&[]).remove(0);
    s.id=id.into();s.title=title.into();s.parent_id=parent.map(str::to_owned);s.runtime_status=runtime.into();s.activity_generation=Some(at);s
}
fn phase(name:&str,at:u64)->PhaseRecord {
    PhaseRecord{phase:name.into(),recorded_at:at,command:None,source:"ACT report".into(),invocation:None,turn_id:None,ended_at:None}
}
fn closed(sessions:&[OverviewSession])->Vec<&str> {
    sessions.iter().filter(|s|s.hidden&&s.runtime_status=="closed").map(|s|s.id.as_str()).collect()
}
#[test]
fn superseded_rounds_passed_stages_and_ended_parents_close_their_children() {
    let _w=common::world();
    let mut parent=session("p","Coordinator",None,"working",100);
    parent.phase_history=vec![phase("Interview",10),phase("Implement",50),phase("Implement",80)];
    let mut sessions=vec![parent,
        session("interview","journal interview",Some("p"),"waiting-parent",40),
        session("dev","journal develop 140",Some("p"),"waiting-parent",60),
        session("dev2","journal develop 141",Some("p"),"waiting-parent",61),
        session("r1","journal review 140",Some("p"),"waiting-parent",62),
        session("r2","journal review 140 r2",Some("p"),"waiting-parent",70),
        session("r3","journal review 140 r3",Some("p"),"waiting-parent",90),
        session("d1","default",Some("p"),"waiting-parent",63),
        session("d2","default",Some("p"),"waiting-parent",64)];
    overview_retire::retire(&mut sessions);
    assert_eq!(closed(&sessions),["interview","r1","r2"],"Only finished stages and earlier rounds close");
    assert!(!sessions[0].hidden,"The coordinator itself is never closed");

    let mut sessions=vec![session("p","Coordinator",None,"closed",100),
        session("c","journal review",Some("p"),"waiting-parent",90),
        session("g","nested",Some("c"),"idle",95),
        session("q","asks",Some("p"),"waiting",96),
        session("w","busy",Some("p"),"working",97)];
    sessions[3].question=Some("Which variant?".into());
    overview_retire::retire(&mut sessions);
    assert_eq!(closed(&sessions),["c","g"],"Children of an ended parent close, but never a question or running work");

    let mut done=session("p","Coordinator",None,"idle",100);done.manual_completion=Some(true);
    let mut sessions=vec![done,session("c","child",Some("p"),"waiting-parent",50)];
    overview_retire::retire(&mut sessions);
    assert_eq!(closed(&sessions),["c"],"A completed assignment releases its subagents");
}
