//! Selecting a project in the native overview shows its Space and starts new agents there without picking an agent.
use crate::common;

#[test]
fn real_programs_native_project_selection_targets_its_space_and_ends_on_another_tab() {
    let _w=common::world();
    let script=r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const values=new Map(), commands=[], handlers=new Map();
let focus={space:'A',tab:'tab-A'};
const list=[{id:'A',name:'tools',folder:'/repo/tools',tabs:[{id:'tab-A'}]},{id:'B',name:'journal',folder:'/repo/journal',tabs:[{id:'tab-B'},{id:'tab-B2'}]}];
const paneList=[{terminal:'t1',tab:'tab-A',cwd:'/repo/tools'},{terminal:'t2',tab:'tab-B',cwd:'/repo/journal'}];
function node(){let id='';const el=new Proxy({},{get(_t,m){return (...a)=>{if(m==='id')id=a[0];if(m==='when'&&a[0])a[1](el);if(String(m).startsWith('on_'))handlers.set(id+':'+m,a.at(-1));return el;};}});return el;}
globalThis.stub={div:node,image:node,v_flex:node,h_flex:node,shared:k=>values.get(k),setShared:(k,v)=>values.set(k,v),focused:()=>focus,
 spaces:()=>list,panes:()=>paneList,run:(c,a)=>{commands.push([c,a]);if(c==='space.select')focus={space:a.id,tab:'tab-'+a.id};},
 knownAgents:async()=>[{key:'claude',name:'Claude Code',icon:'icons/claude.svg',run:['claude'],installed:true,agent:true},{key:'vim',name:'Vim',run:['vim'],installed:true,agent:false}]};
const source=fs.readFileSync(path.join(root,'native/act-overview/project.js'),'utf8').replace(/^import .*;\n/gm,'');
const p=await import(data('const {div,image,v_flex,h_flex,shared,setShared,focused,spaces,panes,run,knownAgents}=globalThis.stub;\n'+source));
const row=(id,terminal)=>({depth:0,session:{id,project:'journal',project_path:'/repo/journal',source:terminal?{Native:{terminal}}:{CodexApp:{thread_id:id}}}});
const journal={name:'journal',path:'/repo/journal',count:2,open:1,rows:[row('desktop',null),row('j','t2')]};
assert.equal(p.projectSpace(journal).id,'B','The Space that runs the project agents');
assert.equal(p.projectSpace({...journal,rows:[row('desktop',null)]}).id,'B','Else the Space opened in the project folder');
assert.equal(p.projectSpace({name:'x',path:'/repo/x',rows:[]}),null);
const cx={notify(){},stop_propagation(){},spawn(fn){return fn(cx);}};
values.set('herdr-act.selected','j');
p.selectProject(journal,cx);
assert.equal(values.get('herdr-act.selected'),null,'Selecting a project clears the agent selection');
assert.deepEqual(commands,[['space.select',{id:'B'}]]);
assert.equal(values.get(p.CHOSEN).name,'journal');
p.followFocus(focus); assert.equal(values.get(p.CHOSEN).tab,'tab-B','The choice adopts the tab its Space shows');
p.followFocus(focus); assert(values.get(p.CHOSEN),'It stays while that tab is shown');
const host={}; p.projectDetail(host,values.get(p.CHOSEN),{},cx); await new Promise(r=>setTimeout(r,0));
assert.deepEqual(host.known.map(a=>a.key),['claude'],'Only installed agent programs are offered');
p.projectDetail(host,values.get(p.CHOSEN),{},cx); handlers.get('start-claude:on_click')();
assert.deepEqual(commands.at(-1),['tab.new',{folder:'/repo/journal',argv:['claude'],space:'B'}],'A new agent starts in the project Space and folder');
p.followFocus({space:'B',tab:'tab-B2'}); assert.equal(values.get(p.CHOSEN),null,'A new or other tab ends the project choice');
p.selectProject({name:'x',path:'/repo/x',count:0,open:0,rows:[]},cx); commands.length=0;
p.projectDetail(host,values.get(p.CHOSEN),{},cx); handlers.get('start-claude:on_click')();
assert.deepEqual(commands,[['space.new',{folder:'/repo/x',argv:['claude']}]],'Without a Space the agent opens one in the project folder');
"#;
    let out=std::process::Command::new("node").args(["--input-type=module","-e",script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
}
