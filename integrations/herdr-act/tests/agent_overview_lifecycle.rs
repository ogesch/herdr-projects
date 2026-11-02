//! Native close controls confirm the target, preserve failures, and never stop a parent for its child.
use crate::common;

#[test]
fn real_programs_native_agent_close_is_targeted_confirmed_and_error_safe() {
    let _w = common::world();
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const {closeSession}=await import(data(fs.readFileSync(path.join(root,'native/act-overview/lifecycle.js'),'utf8')));
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const thread='11111111-1111-4111-8111-111111111111', id='codex-thread:'+thread, binary='/observer';
const child={id,title:'default',parent_id:'parent',source:{CodexApp:{thread_id:thread}},activity_generation:7,runtime_status:'waiting-parent',phase_history:[]};
let calls=[], current=child, fail='', propsValue={}, overlayClosed=[], pending=[];
async function exec(command,args) {
  calls.push([command,args]);
  if(command===binary && args[1]==='native-snapshot') return {code:0,stdout:JSON.stringify({rows:[{session:current}],errors:[]})};
  return {code:fail ? 1 : 0,stderr:fail,stdout:''};
}
await closeSession(child,exec,binary);
assert.deepEqual(calls[1],['codex',['archive',thread]],'Closing a child archives its own thread, never its parent');
assert.deepEqual(JSON.parse(calls[2][1][2]),{id,closed:true,hidden:true,generation:7});
calls=[]; fail='archive rejected';
await assert.rejects(closeSession(child,exec,binary),/archive rejected/);
assert.equal(calls.length,2,'A rejected archive must not hide the row or claim it ended');
calls=[]; fail=''; current={...child,activity_generation:8};
await assert.rejects(closeSession(child,exec,binary),/neue Arbeit/);
assert.equal(calls.length,1,'New work after opening the confirmation must not be closed');
calls=[]; current={...child,source:{Native:{terminal:'remote/t7'}}};
await closeSession(current,exec,binary);
assert.deepEqual(calls[1],['hc',['kill','--','remote/t7']]);
calls=[]; current={...child,source:{CodexApp:{thread_id:'--all'}}};
await assert.rejects(closeSession(current,exec,binary),/Sitzung/);
assert.equal(calls.length,1,'Malformed identifiers must never reach the shutdown command');

const handlers=new Map(), labels=new Map(), values=new Map(); let opened;
function node() {
  let id=''; const el=new Proxy({}, {get(_t,method){return (...args)=>{
    if(method==='id') id=args[0];
    if(method==='accessibility_label') labels.set(id,args[0]);
    if(method==='when' && args[0]) args[1](el);
    if(String(method).startsWith('on_')) handlers.set(id+':'+method,args.at(-1));
    return el;
  };}}); return el;
}
const cx={notify(){},stop_propagation(){},spawn(fn){const p=fn(cx);pending.push(p);return p;}};
globalThis.stub={...model,closeSession,View:class{},div:node,v_flex:node,h_flex:node,theme:()=>({}),
 props:()=>propsValue,shared:k=>values.get(k),setShared:(k,v)=>values.set(k,v),focused:()=>({}),
 openOverlay:v=>{opened=v;},closeOverlay:id=>overlayClosed.push(id),exec,
 terminals:()=>[],spaces:()=>[],panes:()=>[],run(){},setAgentOrder(){},newQuestions(){},renderProjectTree(){}};
const prefix='const {View,div,v_flex,h_flex,theme,props,shared,setShared,focused,openOverlay,closeOverlay,exec,terminals,spaces,panes,run,setAgentOrder,newQuestions,renderProjectTree,runtime,selected,completion,nextCompletion,sourceName,lightSnapshot,liveSnapshot,spaceSnapshot,closeSession}=globalThis.stub;\n';
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'')
 .replace(/^const BINARY = .*;$/m,'const BINARY = "/observer";');
const {default:Overview}=await import(data(prefix+source));
const view=new Overview(); Object.assign(view,{mode:'detail',history:false,error:''});
values.set('herdr-act.selected',id);view.detail([{session:child,group:'Active'}],cx);
assert.equal(labels.get('hide'),'Ausblenden','Inactive children can be hidden without marking the assignment done');
handlers.get('close:on_click')({},cx);
assert.equal(opened.props.mode,'close');assert.equal(opened.props.session.id,id);assert.equal(calls.length,1);
propsValue={...opened.props,overlay:'confirmation'};
const dialog=new Overview();dialog.mode='close';dialog.error='';dialog.render(cx);
handlers.get('cancel:on_click')({},cx);
assert.deepEqual(overlayClosed,['confirmation']);assert.equal(calls.length,1,'Cancel has no shutdown side effects');
calls=[];current=child;fail='archive rejected';overlayClosed=[];
dialog.render(cx);handlers.get('confirm-close:on_key_down')({keystroke:'enter'},cx);
handlers.get('confirm-close:on_click')({},cx);
await Promise.all(pending);
assert.equal(calls.filter(c=>c[0]==='codex').length,1,'Repeated activation must not send shutdown twice');
assert.match(dialog.error,/archive rejected/);assert.deepEqual(overlayClosed,[]);
assert.equal(calls.filter(c=>c[0]===binary && c[1][1]==='native-update').length,0);
calls=[];fail='';pending=[];dialog.render(cx);handlers.get('confirm-close:on_click')({},cx);await Promise.all(pending);
assert.deepEqual(overlayClosed,['confirmation']);
"#;
    let out = std::process::Command::new("node").args(["--input-type=module", "-e", script])
        .arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
