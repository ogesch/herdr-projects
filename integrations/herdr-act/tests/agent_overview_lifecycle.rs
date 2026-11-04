//! Native close controls confirm the target, preserve failures, and never stop a parent for its child.
use crate::common;

#[test]
fn real_programs_native_agent_close_is_targeted_confirmed_and_error_safe() {
    let _w = common::world();
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const {closeSession,closeEffect}=await import(data(fs.readFileSync(path.join(root,'native/act-overview/lifecycle.js'),'utf8')));
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const thread='11111111-1111-4111-8111-111111111111', id='codex-thread:'+thread, binary='/observer';
const child={id,title:'default',parent_id:'parent',source:{CodexApp:{thread_id:thread}},activity_generation:7,runtime_status:'waiting-parent',phase_history:[]};
let others=[], calls=[], current=child, fail='', propsValue={}, overlayClosed=[], pending=[];
async function exec(command,args) {
  calls.push([command,args]);
  if(command===binary && args[1]==='native-snapshot') return {code:0,stdout:JSON.stringify({rows:[{session:current},...others],errors:[]})};
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
const parent={id:'parent',title:'Coordinator',source:{Native:{terminal:'t1z'}},phase_history:[]};
calls=[]; others=[{session:parent}]; current={...child,source:{Native:{terminal:'t1z'}}};
await closeSession(current,exec,binary);
assert.deepEqual(calls.map(c=>c[0]),[binary,binary],'A subagent inside its parent terminal closes in the overview only');
assert.deepEqual(JSON.parse(calls[1][1][2]),{id,closed:true,hidden:true,generation:7});
assert.match(closeEffect(current,[{session:parent},{session:current}]),/Hauptagent läuft weiter/);
assert.match(closeEffect({...child,parent_id:null,source:{Native:{terminal:'t1z'}}},[]),/Terminal/);
calls=[]; others=[]; current={...child,source:'Ended'};
await closeSession(current,exec,binary);
assert.deepEqual(calls.map(c=>c[0]),[binary,binary],'An ended session with nothing to stop still leaves the list');
current=child; calls=calls.slice(0,1); // the detail checks below count new calls after one earlier call

const handlers=new Map(), labels=new Map(), values=new Map(), texts=new Map(), colors=new Map(), backgrounds=new Map(); let opened;
const links=new Set(), hrefs=new Map();
function node() {
  let id=''; const el=new Proxy({}, {get(_t,method){return (...args)=>{
    if(method==='id') id=args[0];
    if(method==='accessibility_label') labels.set(id,args[0]);
    if(method==='child' && typeof args[0]==='string') texts.set(id,args[0]);
    if(method==='text_color') colors.set(id,args[0]);
    if(method==='bg') backgrounds.set(id,args[0]);
    if(method==='href') hrefs.set(id,args[0]);
    if(method==='flex_grow') assert.equal(typeof args[0],'number','Native flex growth requires an explicit weight');
    if(method==='when' && args[0]) args[1](el);
    if(String(method).startsWith('on_')) handlers.set(id+':'+method,args.at(-1));
    return el;
  };}}); return el;
}
const cx={notify(){},stop_propagation(){},spawn(fn){const p=fn(cx);pending.push(p);return p;}};
globalThis.stub={...model,closeSession,closeEffect,View:class{},div:node,v_flex:node,h_flex:node,theme:()=>({}),
 Link:{new:id=>{links.add(id);return node().id(id);}},image:node,
 props:()=>propsValue,shared:k=>values.get(k),setShared:(k,v)=>values.set(k,v),focused:()=>({}),
 openOverlay:v=>{opened=v;},closeOverlay:id=>overlayClosed.push(id),exec,
 terminals:()=>[],spaces:()=>[],panes:()=>[],run(){},setAgentOrder(){},newQuestions(){},renderProjectTree(){}};
const summarySource=fs.readFileSync(path.join(root,'native/act-overview/session-summary.js'),'utf8').replace(/^import .*;\n/gm,'');
const {sessionSummary}=await import(data('const {div,image,v_flex,h_flex,Link,runtime,completion,sourceName,programOf}=globalThis.stub;\n'+summarySource));
globalThis.stub.sessionSummary=sessionSummary;
const palette={ansi1:'#c00000',ansi2:'#008000',ansi3:'#806000',ansi4:'#0055cc',muted:'#666666',dim:'#999999',foreground:'#111111',chrome:'#eeeeee'};
for(const [status,color] of [['working',palette.ansi4],['active',palette.ansi4],['blocked',palette.ansi3],['waiting',palette.ansi3],['waiting-parent',palette.ansi3],['error',palette.ansi1],['idle',palette.muted],['done',palette.muted],['unknown',palette.muted]]) {
  sessionSummary({...child,runtime_status:status,task_complete:true,act_phase:'Implement',ticket:'https://github.com/ogesch/cheekily/issues/128'},palette);
  assert.equal(colors.get('session-runtime'),color);
  assert.equal(texts.get('session-completion'),'Auftrag: Fertig');
  assert.equal(colors.get('session-completion'),palette.ansi2,'Completion stays separate from runtime');
  assert.equal(colors.get('session-phase'),palette.foreground,'A known ACT phase reads as plain text');
  assert.equal(texts.get('session-ticket'),'https://github.com/ogesch/cheekily/issues/128');
}
sessionSummary({...child,task_complete:true,manual_completion:false},palette);
assert.equal(texts.get('session-completion'),'Auftrag: Nicht abgeschlossen','Manual completion still overrides detection');
sessionSummary({...child,runtime_status:'new-status'},{...palette,muted:'var(--muted)'});
assert.equal(texts.get('session-completion'),'Auftrag: Unbekannt');
assert.equal(texts.get('session-phase'),'ACT: Unbekannt'); assert.equal(colors.get('session-phase'),palette.dim);
assert.equal(texts.get('session-ticket'),'Unbekannt');
assert.equal(backgrounds.get('session-runtime'),undefined,'Facts are text on the surface, not tinted badges');
for(const url of ['https://github.com/ogesch/cheekily/issues/128','http://localhost:8080/ticket/128']) {
  links.clear();hrefs.clear();sessionSummary({...child,ticket:url},palette);
  assert(links.has('session-ticket'),'Web tickets use the native external-browser link');
  assert.equal(hrefs.get('session-ticket'),url);
  assert.equal(labels.get('session-ticket'),'Ticket im Browser öffnen: '+url);
}
for(const ticket of [undefined,'#128','docs/spec.md','javascript:alert(1)','file:///tmp/spec.md','https://','https://example.com/invalid path']) {
  links.clear();hrefs.clear();sessionSummary({...child,ticket},palette);
  assert(!links.has('session-ticket'),'Missing, local, malformed, or non-web tickets stay plain text');
  assert(!hrefs.has('session-ticket'));
  assert.equal(texts.get('session-ticket'),ticket??'Unbekannt');
}
const closeSource=fs.readFileSync(path.join(root,'native/act-overview/close-view.js'),'utf8').replace(/^import .*;\n/gm,'');
Object.assign(globalThis.stub,await import(data('const {div,v_flex,h_flex,theme,props,shared,exec,openOverlay,closeOverlay,closeSession,closeEffect}=globalThis.stub;\n'+closeSource)));
const prefix='const {closeDialog,agentMenu,openClose,openMenu,View,div,v_flex,h_flex,theme,props,shared,setShared,focused,openOverlay,closeOverlay,exec,terminals,spaces,panes,run,setAgentOrder,newQuestions,renderProjectTree,runtime,selected,completion,nextCompletion,sourceName,lightSnapshot,liveSnapshot,spaceSnapshot,embeddedChild,ownerRow,closeSession,sessionSummary}=globalThis.stub;\n';
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'').replace(/^/,'const {CHOSEN,chooseProject,followFocus,projectDetail}={CHOSEN:"herdr-act.chosen-project",chooseProject(){},followFocus(){},projectDetail(){}};\n')
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
propsValue={mode:'menu',session:child,overlay:'menu'};overlayClosed=[];opened=null;
const menu=new Overview();menu.mode='menu';menu.render(cx);
assert.equal(labels.get('menu-close'),'Unteragent schließen …');
handlers.get('menu-close:on_click')({},cx);
assert.deepEqual(overlayClosed,['menu']);assert.equal(opened.props.mode,'close','The context menu leads to the same confirmation');
"#;
    let out = std::process::Command::new("node").args(["--input-type=module", "-e", script])
        .arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
