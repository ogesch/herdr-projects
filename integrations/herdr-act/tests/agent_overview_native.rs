//! Native Herdr identities, core subagents, hook joins, and correction migration share truthful task status.
use crate::common;
use sw::{agents::{overview::{OverviewGroup,SessionSource},overview_native,overview_events::{self as events,Activity},overview_store::OverviewStore},json::json};
#[test]
fn native_app_agents_keep_completion_separate_and_join_hook_and_core_children() {
    let w=common::world();
    let app=json!({"spaces":[{"tabs":[{"terminals":[
        {"terminal":"t1","app":"codex","status":"done","sub_agents":["t2"]},
        {"terminal":"t3","app":"codex","status":"blocked"},
        {"terminal":"t4","app":null,"status":null}
    ]}]}]});
    let details=vec![json!({"address":"t1","title":"Native work","cwd":w.root}),json!({"address":"t2","cwd":w.root,"status":{"app":"claude-code","state":"idle"}})];
    let mut sessions=overview_native::from_app(&app,&details);
    assert_eq!(sessions.len(),3);assert_eq!(sessions[0].group,OverviewGroup::Unknown);
    assert!(sessions[0].task_complete.is_none());assert_eq!(sessions[2].group,OverviewGroup::NeedYou);
    assert_eq!(sessions[1].runtime_status,"waiting-parent");
    assert!(matches!(&sessions[0].source,SessionSource::Native{terminal} if terminal=="t1"));
    let mut store=OverviewStore::at(w.base.join("corrections.json"));store.correction("core-terminal:t1").title=Some("Corrected".into());
    let activity=Activity{key:"codex-thread:native-root".into(),provider:"codex".into(),session:"native-root".into(),core_terminal:Some("t1".into()),runtime:"idle".into(),..Default::default()};
    events::project::apply(&mut sessions,&[activity]);
    assert_eq!(sessions[0].id,"codex-thread:native-root");assert_eq!(sessions[1].parent_id.as_deref(),Some("codex-thread:native-root"));
    assert!(store.apply(&mut sessions[0]));assert_eq!(sessions[0].title,"Corrected");
    let mut child=Activity{key:"claude-session:native-child".into(),provider:"claude".into(),session:"native-child".into(),core_terminal:Some("t2".into()),..Default::default()};
    child.pending.insert("question".into(),events::Pending{kind:"reported_question".into(),text:"Which variant?".into()});
    events::project::apply(&mut sessions,&[child]);
    assert_eq!(sessions[1].runtime_status,"waiting");assert_eq!(sessions[0].runtime_status,"done");
}

#[test]
fn native_core_messages_expose_the_task_and_act_command_without_transport_markup() {
    let input=json!({"hook_event_name":"UserPromptSubmit","session_id":"child","prompt":"<hc from=\"t1\">$land-feature\nA bounded task</hc>"});
    let event=events::normalize::from_hook("codex",&input,1).unwrap().remove(0);
    assert_eq!(event.title.as_deref(),Some("$land-feature"));
    assert_eq!(event.phase.as_deref(),Some("Land Feature"));
}

#[test]
fn real_programs_native_mod_model_keeps_hidden_families_and_native_selection_consistent() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import assert from 'node:assert/strict';
const m=await import('data:text/javascript;base64,'+Buffer.from(fs.readFileSync(process.argv[1],'utf8')).toString('base64'));
const rows=[{session:{id:'a',project:'P',hidden:true}},{session:{id:'b',project:'P',parent_id:'a'}},{session:{id:'c',project:'Q',manual_completion:true,source:{Native:{terminal:'t1'}}}}];
assert.equal(m.filtered(rows,'',false).length,1); assert.equal(m.filtered(rows,'P',true).length,2);
rows[0].session.hidden=false; assert.equal(m.filtered(rows,'',false,new Set(['a'])).length,2);
assert.equal(m.selected(rows,'gone','t1').session.id,'c'); assert.equal(m.nextCompletion(rows[2].session),false);
assert.equal(m.runtime('done'),'Inaktiv');
rows.push({session:{id:'d',source:{Native:{terminal:'t2'}}}});
assert.equal(m.selected(rows,'c','t2').session.id,'d');
const history=[{depth:0,session:{id:'A'}},{depth:1,session:{id:'a'}},{depth:0,session:{id:'B'}},{depth:1,session:{id:'b'}},{depth:0,session:rows[2].session}];
const page=m.pageFamilies(history,1,1); assert.equal(page.pages,2);
assert.deepEqual(page.rows.map(r=>r.session.id),['c','B','b']);
const light=m.lightSnapshot({rows:history,errors:[]},'A','t1');
assert.deepEqual(light.rows.map(r=>r.session.id),['A','c']); assert.deepEqual(light.projects,['Q']);"#;
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("native/act-overview/model.js");
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(path).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}

#[test]
fn real_programs_native_tab_navigation_keeps_shells_and_agent_panes_selectable() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], layout=JSON.parse(fs.readFileSync(path.join(root,'native/act-workflow.json'),'utf8'));
const find=n=>n.id==='act-terminal'?n:(n.children??[]).map(find).find(Boolean);
assert.deepEqual(find(layout).content,{tab:'active'},'Tab navigation must show all panes of the active tab');
const m=await import('data:text/javascript;base64,'+Buffer.from(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')).toString('base64'));
const a={session:{id:'a',source:{Native:{terminal:'t1'}}}}, b={session:{id:'b',source:{Native:{terminal:'t2'}}}};
const desktop={session:{id:'desktop',source:{CodexApp:{thread_id:'desktop'}}}}, rows=[a,b,desktop];
assert.equal(m.selected(rows,'a','t2'),b);
assert.equal(m.selected(rows,'a','shell'),undefined,'Shell tabs must not keep the previous agent details');
assert.equal(m.selected(rows,null,'shell'),undefined);
assert.equal(m.selected(rows,'a',null),undefined,'Mod panes have no focused terminal');
assert.equal(m.selected(rows,'desktop','t1'),desktop,'An explicitly opened Desktop conversation keeps its own details');
assert.equal(m.selected(rows,null,'t1'),a,'After clearing the external selection, follow the active native pane');"#;
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}

#[test]
fn real_programs_native_desktop_sessions_open_in_herdr_without_sending_a_prompt() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const values=new Map(), commands=[], calls=[], pending=[];
const id='01a18799-bdea-75b3-825c-f35b974fa76d';
const session={id:'codex-thread:'+id,title:'Existing chat',project_path:"/tmp/Oliver's project",source:{CodexApp:{thread_id:id}}};
const row={session}, snapshot={rows:[row],errors:[]};
const cx={notify(){},spawn(f){pending.push(f(cx));}};
globalThis.stub={run:(...args)=>commands.push(args),setShared:(k,v)=>values.set(k,v),shared:k=>values.get(k),focused:()=>({terminal:'original'}),panes:()=>[],lightSnapshot:model.lightSnapshot,liveSnapshot:model.liveSnapshot,live:[{address:'existing',state:'live'}],
 exec:async(...args)=>{calls.push(args);return {code:0,stdout:JSON.stringify({terminal:'created'}),stderr:''};}};
const prefix='class View {}\nconst {run,setShared,shared,focused,panes,lightSnapshot,liveSnapshot}=globalThis.stub; const terminals=()=>globalThis.stub.live; const exec=(...args)=>globalThis.stub.exec(...args);\n';
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'').replace(/^/,'const {CHOSEN,chooseProject,followFocus,projectDetail}={CHOSEN:"herdr-act.chosen-project",chooseProject(){},followFocus(){},projectDetail(){}};\n');
const {default:Overview}=await import(data(prefix+source)), view=new Overview();
Object.assign(view,{snapshot,opening:false,error:''});
view.choose(row,cx);
assert.equal(calls.length,0,'Selecting a saved chat must not switch to Desktop');
assert(commands.some(([name])=>name==='layout.hide'));
commands.length=0;
view.openInHerdr(session,cx); view.openInHerdr(session,cx); await Promise.all(pending.splice(0));
assert.equal(calls.length,1,'A double click must open only one terminal');
assert.deepEqual(calls[0].slice(0,1),['herdr-app']);
assert.deepEqual(calls[0][1].slice(0,2),['do','terminal.open']);
assert.deepEqual(JSON.parse(calls[0][1][2]),{argv:['codex','resume','--no-daemon',id],folder:session.project_path,name:session.title});
assert(commands.some(([name,args])=>name==='agent.focus' && args.terminal==='created'));
assert(commands.some(([name])=>name==='layout.show')); assert.equal(values.get('herdr-act.selected'),null);
assert.equal(view.error,''); assert.equal(view.opening,false);
calls.length=0; commands.length=0;
values.set('herdr-act.snapshot',{rows:[{session:{...session,source:{Native:{terminal:'existing'}}}}]});
view.openInHerdr(session,cx); await Promise.all(pending.splice(0));
assert.equal(calls.length,0,'Reuse an existing Herdr session with the same confirmed identity');
assert(commands.some(([name,args])=>name==='agent.focus' && args.terminal==='existing'));
calls.length=0; commands.length=0; globalThis.stub.live=[];
view.openInHerdr(session,cx); await Promise.all(pending.splice(0));
assert.equal(calls.length,1,'A closed cached terminal must be opened again rather than focused');
assert(!commands.some(([name,args])=>name==='agent.focus' && args.terminal==='existing'));
values.set('herdr-act.snapshot',snapshot); commands.length=0;
globalThis.stub.exec=async()=>({code:1,stderr:'Cannot open project'});
view.openInHerdr(session,cx); await Promise.all(pending.splice(0));
assert.equal(view.error,'Cannot open project'); assert.equal(view.opening,false);
assert(!commands.some(([name])=>name==='agent.focus'));
globalThis.stub.exec=async()=>{throw new Error('Command unavailable');};
view.openInHerdr(session,cx); await Promise.all(pending.splice(0));
assert.equal(view.error,'Command unavailable'); assert.equal(view.opening,false);
globalThis.stub.exec=async(...args)=>{calls.push(args);return {code:0,stderr:''};};
view.openInDesktop(session,cx); await Promise.all(pending.splice(0));
assert.deepEqual(calls.at(-1),['/usr/bin/open',['codex://threads/'+id]]);"#;
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}

#[test]
fn real_programs_native_overview_ignores_closed_terminals_and_exposes_spaces() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const row=(id,terminal,extra={})=>({depth:0,session:{id,source:{Native:{terminal}},...extra}});
const live=row('live','t1'), closed=row('closed','t2');
const desktop={depth:0,session:{id:'desktop',source:{CodexApp:{thread_id:'desktop'}}}};
const snapshot={rows:[live,closed,desktop],errors:[]}, commands=[], values=new Map();
let refreshes=0;
globalThis.stub={run:(...args)=>commands.push(args),setShared:(k,v)=>values.set(k,v),shared:k=>values.get(k),
focused:()=>({terminal:'t1'}),panes:()=>[{terminal:'t1',pane:'p1'}],live:[{address:'t1',state:'live'},{address:'t2',state:'finished'}],
lightSnapshot:model.lightSnapshot,liveSnapshot:model.liveSnapshot};
const prefix='class View{}; const {run,setShared,shared,focused,panes,lightSnapshot,liveSnapshot}=globalThis.stub; const terminals=()=>globalThis.stub.live;\n';
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'').replace(/^/,'const {CHOSEN,chooseProject,followFocus,projectDetail}={CHOSEN:"herdr-act.chosen-project",chooseProject(){},followFocus(){},projectDetail(){}};\n');
const {default:Overview}=await import(data(prefix+source)); const view=new Overview();
Object.assign(view,{snapshot,refresh:()=>refreshes++});
view.choose(closed,{});
assert.deepEqual(commands,[],'A stale row must never peek or focus a closed terminal');
assert.equal(values.size,0,'A stale click must not change selection or shared context');
assert.equal(refreshes,1);
view.choose(live,{});
assert(commands.some(([name,args])=>name==='pane.focus'&&args.pane==='p1'));
assert(!commands.some(([name])=>name==='terminal.peek'));
commands.length=0; globalThis.stub.live.push({address:'t3',state:'live'});
view.choose(row('child','t3'),{});
assert(commands.some(([name,args])=>name==='terminal.peek'&&args.terminal==='t3'),'A live agent without a pane remains accessible');
const pruned=model.liveSnapshot(snapshot,globalThis.stub.live);
assert.deepEqual(pruned.rows.map(r=>r.session.id),['live','desktop']);
assert.equal(snapshot.rows.length,3,'Pruning must not mutate stored history');
assert.equal(model.liveSnapshot(pruned,globalThis.stub.live),pruned,'Keep the render cache when nothing changed');
const child={...row('child','t3',{parent_id:'closed'}),depth:1};
const orphan=model.liveSnapshot({rows:[closed,child]},[{address:'t3',state:'live'}]);
assert.equal(orphan.rows[0].depth,0); assert.equal(orphan.rows[0].session.parent_id,null);
const hiddenRoot=row('hidden','t2',{hidden:true});
const hiddenChild={...row('hidden-child','t3',{parent_id:'hidden',hidden:false}),depth:1};
const grandchild={...row('grandchild','t4',{parent_id:'hidden-child',hidden:false}),depth:2};
const hiddenFamily=model.liveSnapshot({rows:[hiddenRoot,hiddenChild,grandchild]},[{address:'t3',state:'live'},{address:'t4',state:'live'}]);
assert.deepEqual(model.filtered(hiddenFamily.rows,'',false),[],'Closing a parent must not reveal hidden descendants');
assert.deepEqual(hiddenFamily.rows.map(r=>r.depth),[0,1],'Rebase the whole surviving subtree');
assert.equal(hiddenFamily.rows[1].session.parent_id,'hidden-child');
assert.equal(grandchild.depth,2); assert.equal(hiddenChild.session.hidden,false,'Keep the stored snapshot unchanged');
const closedMiddle={...row('middle','t2',{parent_id:'live',hidden:true}),depth:1};
const leaf={...row('leaf','t3',{parent_id:'middle',hidden:false}),depth:2};
const reparented=model.liveSnapshot({rows:[live,closedMiddle,leaf]},[{address:'t1',state:'live'},{address:'t3',state:'live'}]);
assert.equal(reparented.rows[1].session.parent_id,'live'); assert.equal(reparented.rows[1].depth,1);
assert.deepEqual(model.filtered(reparented.rows,'',false).map(r=>r.session.id),['live']);
const layout=JSON.parse(fs.readFileSync(path.join(root,'native/act-workflow.json'),'utf8'));
const find=(n,id)=>n.id===id?n:(n.children??[]).map(c=>find(c,id)).find(Boolean);
const lists=find(layout,'sidebar-lists').children;
assert(lists.findIndex(n=>n.content?.mod==='act-spaces')>=0,'The native Space controls must be visible');
assert(lists.findIndex(n=>n.content?.mod==='act-spaces')<lists.findIndex(n=>n.content?.mod==='act-overview'));
"#;
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}

#[test]
fn real_programs_project_tree_preserves_agent_status_families_and_closed_history() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const data = s => 'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const model = data(fs.readFileSync(process.argv[1],'utf8'));
const tree = await import(data(fs.readFileSync(path.join(path.dirname(process.argv[1]),'tree-model.js'),'utf8').replace('"./model.js"',JSON.stringify(model))));
const row=(id,project,runtime,group='Unknown',extra={})=>({depth:0,group,session:{id,title:id,program:'codex',project,runtime_status:runtime,source:{Native:{terminal:id}},...extra}});
const working=row('main','Journal','working','NeedYou');
const question={...row('child','Journal','waiting','NeedYou',{parent_id:'main',question:'Which variant?',activity_generation:1}),depth:1};
const idle=row('idle','Journal','done');
const problem=row('problem','Katalog','error','Problems',{problem:'Build failed'});
const unknown=row('unknown','Herdr','unknown');
const saved=row('saved','Journal','notLoaded','Unknown',{source:{CodexApp:{thread_id:'saved'}}});
const done=row('done','Journal','idle','Done',{task_complete:true});
const hidden=row('hidden','Journal','working','Active',{hidden:true});
const hiddenChild={...row('hidden-child','Journal','working','Active',{parent_id:'hidden'}),depth:1};
const rows=[working,question,idle,problem,unknown,saved,done,hidden,hiddenChild];
const forest=tree.projectTree(rows);
assert.equal(forest.projects.reduce((n,p)=>n+p.count,0),5,'Completed assignments stay counted in their project');
assert.equal(forest.projects[0].name,'Journal'); assert.equal(forest.projects[0].status.kind,'question');
assert.equal(forest.projects[0].rows.length,4); assert.equal(tree.statusOf(working.session).kind,'working');
assert.equal(forest.projects[0].rows.at(-1).session.id,'done','A completed family stays in its project, after the open ones');
assert.equal(tree.projectTree([done]).projects[0].status.kind,'done','Only a project with nothing open shows the check');
assert.equal(tree.projectTree([idle,done]).projects[0].status.kind,'idle','A completed family never marks an open project as done');
assert.equal(tree.statusOf(idle.session).kind,'idle'); assert.equal(tree.statusOf(unknown.session).kind,'unknown');
assert.equal(forest.done,undefined,'There is no separate Fertig section'); assert.equal(forest.saved.length,1);
assert.deepEqual(tree.projectTree(rows,{project:'Katalog'}).projects.map(p=>p.name),['Katalog']);
assert.equal(tree.projectTree(rows,{showHidden:true}).projects[0].count,4);
const attention=tree.newQuestions(rows,new Set());
assert(attention.agents.has('main') && attention.agents.has('child')); assert(attention.projects.has('Journal'));
assert.equal(tree.newQuestions(rows,attention.keys).agents.size,0);
const other={...question,session:{...question.session,project:'Other'}};
const related=tree.projectTree([working,other],{project:'Other'});
assert.equal(related.projects[0].name,'Journal'); assert.equal(related.projects[0].rows.length,2);
assert.equal(tree.phaseText(null),'Unbekannt','Missing ACT data reads unknown, never blank'); assert.equal(tree.phaseText('Create Issues'),'Issues');
assert.equal(tree.commandText({phase_history:[]}),'Unbekannt'); assert.equal(tree.commandText({}),'Unbekannt');
assert.equal(tree.statusOf({runtime_status:'waiting-parent'}).icon,'waiting','Waiting for the parent has its own status mark');
const oldChild={...row('old-child','Journal','done','Done',{parent_id:'main',task_complete:true}),depth:1};
const cycleA=row('cycle-a','Journal','idle','Unknown',{parent_id:'cycle-b'});
const cycleB=row('cycle-b','Journal','working','Active',{parent_id:'cycle-a'});
const sibling={...oldChild,session:{...oldChild.session,id:'sibling'}};
const nested={...oldChild,depth:2,session:{...oldChild.session,id:'nested',parent_id:'old-child'}};
const guides=tree.branchGuides([working,oldChild,nested,sibling]);
assert.deepEqual(guides.get('main'),[]);
assert.deepEqual(guides.get('old-child'),['branch']);
assert.deepEqual(guides.get('nested'),['stem','last'],'Continue the ancestor line past a nested last child');
assert.deepEqual(guides.get('sibling'),['last'],'Stop the line at the final sibling');
assert.deepEqual(tree.branchGuides([working,oldChild,nested]).get('nested'),['blank','last']);
assert.deepEqual(tree.branchGuides([working,sibling]).get('sibling'),['last'],'Recompute after filtering or folding');
assert.equal(tree.branchGuides([cycleA,cycleB]).size,2,'Cyclic metadata must not loop');
assert.equal(tree.commandText({phase_history:[{command:'act-interview'},{command:'act-implement-flutter'}]}),'implement-flutter');"#;
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("native/act-overview/model.js");
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(path).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}
