//! Native ACT space scoping keeps terminal ownership, family visibility, and the global view consistent.
use crate::common;

#[test]
fn real_programs_native_space_scope_preserves_families_and_worktree_context() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const m=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const context=await import(data(fs.readFileSync(path.join(root,'native/act-spaces/context.js'),'utf8')));
const row=(id,terminal,parent=null,depth=0)=>({depth,session:{id,parent_id:parent,project:'same-repo',source:terminal?{Native:{terminal}}:{CodexApp:{thread_id:id}}}});
const a=row('a','t1'), child=row('child','child-terminal','a',1), grand=row('grand',null,'child',2);
const b=row('b','t2'), moved=row('moved','t3','a',1), nested=row('nested',null,'moved',2);
a.session.hidden=true;
const remote=row('remote','server/t1'), desktop=row('desktop',null);
const cycle=row('cycle',null,'cycle-2'), cycle2=row('cycle-2',null,'cycle');
const snapshot={rows:[a,child,grand,b,moved,nested,remote,desktop,cycle,cycle2],errors:['source warning']};
const panes=[{terminal:'t1',tab:'A'},{terminal:'t2',tab:'B'},{terminal:'t3',tab:'B'},{terminal:'server/t1',tab:'server/A'},{terminal:null,tab:'A'}];
const first=m.spaceSnapshot(snapshot,{id:'space-a',tabs:[{id:'A'}]},panes);
assert.deepEqual(first.rows.map(r=>r.session.id),['a','child','grand'],'A shared repository must not include another Space');
assert.deepEqual(first.projects,['same-repo']); assert.deepEqual(first.errors,snapshot.errors);
const second=m.spaceSnapshot(snapshot,{id:'space-b',tabs:[{id:'B'}]},panes);
assert.deepEqual(second.rows.map(r=>r.session.id),['b','moved','nested'],'A child with its own Space follows that terminal');
assert.equal(second.rows[1].depth,0); assert.equal(second.rows[1].session.parent_id,null);
assert.equal(second.rows[2].depth,1); assert.equal(second.rows[1].session.hidden,true);
assert.equal(moved.depth,1); assert.equal(moved.session.parent_id,'a','Scoping must not mutate source data');
assert.deepEqual(m.spaceSnapshot(snapshot,{tabs:[{id:'server/A'}]},panes).rows.map(r=>r.session.id),['remote']);
assert.deepEqual(m.spaceSnapshot(snapshot,undefined,panes).rows,[]);
assert.equal(m.spaceSnapshot(snapshot,undefined,panes,true),snapshot,'All Spaces keeps unassigned desktop history');
const w={name:'tools',folder:'/repo/old',tabs:[{id:'A'}]}, cwd=context.place(w,[{tab:'B',cwd:'/other'},{tab:'A',cwd:'/repo/cheekily/worktrees/login'}]);
assert.equal(cwd,'/repo/cheekily/worktrees/login');
assert.deepEqual(context.describeSpace({w,g:{name:'cheekily',branch:'feat/login'},cwd,known:true}),{repository:'Repo: cheekily · feat/login',folder:cwd});
assert.equal(context.describeSpace({w,g:null,known:false}).repository,'Repo: unbekannt');
assert.equal(context.describeSpace({w,g:null,known:true}).repository,'Kein Git-Repo');
assert.equal(context.place(w,[]),w.folder);
"#;
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}

#[test]
fn real_programs_native_space_selection_and_global_toggle_stay_in_sync() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const context=await import(data(fs.readFileSync(path.join(root,'native/act-spaces/context.js'),'utf8')));
const handlers=new Map(), values=new Map(), commands=[], labels=new Map();
function node() {
  let id='';
  const element=new Proxy({}, {get(_target,method) { return (...args)=>{
    if(method==='id') id=args[0];
    if(method==='accessibility_label') labels.set(id,args[0]);
    if(method==='when' && args[0]) args[1](element);
    if(String(method).startsWith('on_')) handlers.set(id+':'+method+':'+(typeof args[0]==='string'?args[0]:''),args.at(-1));
    return element;
  }; }});
  return element;
}
const list=[{id:'A',name:'One',tabs:[{id:'tab-A'}]},{id:'B',name:'Two',tabs:[{id:'tab-B'}]}];
const paneList=[{terminal:'t1',tab:'tab-A'},{terminal:'t2',tab:'tab-B'}];
const row=(id,terminal)=>({depth:0,session:{id,project:'same-repo',source:{Native:{terminal}}}});
const snapshot={rows:[row('a','t1'),row('b','t2'),{depth:0,session:{id:'desktop',source:{CodexApp:{thread_id:'desktop'}}}}],errors:[]};
let focus={space:'A',tab:'tab-A',terminal:'t1'}, rendered=[];
const cx={notify(){},stop_propagation(){}};
globalThis.stub={...model,View:class{},div:node,v_flex:node,h_flex:node,svg:node,
 shared:key=>values.get(key),setShared:(key,value)=>values.set(key,value),
 focused:()=>focus,spaces:()=>list.map(s=>({...s,active:s.id===focus.space})),panes:()=>paneList,
 terminals:()=>paneList.map(p=>({address:p.terminal,state:'live'})),theme:()=>({}),props:()=>({}),machines:()=>[],
 layout:()=>({id:'sidebar',size:{px:400}}),setAgentOrder(){},
 run(command,args){commands.push([command,args]);if(command==='space.select')focus={space:args.id,tab:'tab-'+args.id,terminal:args.id==='A'?'t1':'t2'};},
 renderProjectTree(host,scoped){host.shownRows=scoped.rows;rendered=scoped.rows.map(r=>r.session.id);return node();}};
globalThis.events=await import(data(fs.readFileSync(path.join(root,'native/act-overview/refresh-events.js'),'utf8').replace(/^import .*;\n/gm,'const {watchFolder,shared,setShared}=globalThis.stub;\n')));
const prefix='const {nativeKey,queueRefresh,watchError}=globalThis.events;\nconst {View,div,v_flex,h_flex,svg,shared,setShared,focused,spaces,panes,terminals,theme,props,machines,layout,setAgentOrder,run,renderProjectTree,liveSnapshot,spaceSnapshot,selected}=globalThis.stub;\n';
const overview=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'');
const {default:Overview}=await import(data(prefix+overview)), view=new Overview();
Object.assign(view,{mode:'list',snapshot,tab:'tab-A',space:'A',pages:{},error:''});
view.render(cx);assert.deepEqual(rendered,['a']);
assert(!handlers.has('scope-space:on_click:') && !handlers.has('scope-all:on_click:'),'The agent list has no Space navigation controls');
const spaceSource=fs.readFileSync(path.join(root,'native/act-spaces/view.js'),'utf8').replace(/^import .*;\n/gm,'');
globalThis.stub.describeSpace=context.describeSpace;
const {renderSpaces}=await import(data(prefix+'const {describeSpace}=globalThis.stub;\n'+spaceSource));
const host={pressed:null,over:null,machines:[],folded:new Set(),shown:new Set(['A','B']),focus:new Map([['A',{}],['B',{}]]),allFocus:{},
 ordered:()=>globalThis.stub.spaces().map((w,i)=>({w,i,g:null,tree:null,cwd:'/repo',known:true}))};
renderSpaces(host);assert.equal(labels.get('space-all'),'All Spaces');
view.pages.Saved=2;
handlers.get('space-all:on_click:')({},cx);view.render(cx);assert.deepEqual(rendered,['a','b','desktop']);assert.deepEqual(view.pages,{});
handlers.get('space-A:on_mouse_down:left')({},cx);handlers.get('space-A:on_mouse_up:left')({},cx);
view.render(cx);assert.deepEqual(rendered,['a'],'Clicking the already active Space also leaves global mode');
handlers.get('space-all:on_click:')({},cx);view.render(cx);
values.set('herdr-act.project','stale-project');values.set('herdr-act.selected','desktop');
handlers.get('space-B:on_mouse_down:left')({},cx);handlers.get('space-B:on_mouse_up:left')({},cx);
assert.equal(values.get('herdr-act.all-spaces'),false);assert.equal(values.get('herdr-act.project'),'');assert.equal(values.get('herdr-act.selected'),null);
view.render(cx);assert.deepEqual(rendered,['b'],'Clicking a Space leaves global mode and filters immediately');
handlers.get('space-all:on_key_down:')({keystroke:'enter'},cx);view.render(cx);assert.deepEqual(rendered,['a','b','desktop']);
handlers.get('space-B:on_key_down:')({keystroke:'enter'},cx);view.render(cx);assert.deepEqual(rendered,['b']);
"#;
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}
