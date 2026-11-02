//! ACT views react to source changes without five-second polling and coalesce event bursts without losing updates.
use crate::common;

#[test]
fn real_programs_file_events_filter_noise_and_preserve_coalesced_updates() {
    let _w = common::world();
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const values=new Map(), watches=new Map(), timers=[];
Object.assign(globalThis,{
 shared:key=>values.get(key),setShared:(key,value)=>values.set(key,value),
 watchFolder:root=>new Promise((resolve,reject)=>{const calls=watches.get(root)??[]; calls.push({resolve,reject}); watches.set(root,calls);}),
});
const events=await import(data(fs.readFileSync(path.join(root,'native/act-overview/refresh-events.js'),'utf8').replace(/^import .*;\n/gm,'')));
const {classifyChanges:classify,nativeKey,sourceChanged,queueRefresh,snapshotReceived,watchSources,conversationChanged,watchError,FILE_EVENTS}=events;
const a='11111111-1111-1111-1111-111111111111', b='22222222-2222-2222-2222-222222222222', c='33333333-3333-3333-3333-333333333333';
const rollout=id=>'/codex/sessions/2026/11/02/rollout-2026-'+id+'.jsonl';
assert.deepEqual(classify('codex','/codex',[rollout(a)],[a]),{snapshot:false,catalog:false,all:false,threads:[a]});
assert.equal(classify('codex','/codex',[rollout(b)],[a]).catalog,true);
for(const p of ['/codex/state_5.sqlite-shm','/codex/log/codex.log','/codex/plugins/cache.json','/elsewhere/state_5.sqlite'])
 assert.equal(classify('codex','/codex',[p]).snapshot,false,p);
for(const p of ['/codex/state_5.sqlite','/codex/state_5.sqlite-wal','/codex/session_index.jsonl','/codex/app-server-control/app-server-control.sock'])
 assert.equal(classify('codex','/codex',[p]).catalog,true,p);
assert.deepEqual(classify('codex','/codex',['/codex']),{snapshot:true,catalog:true,all:true,threads:[]});
for(const p of ['/data/agent-overview.json','/data/agent-overview/sessions/hook.json','/data/agent-overview/codex-cache/snapshot.json'])
 assert.equal(classify('overview','/data',[p]).snapshot,true,p);
for(const p of ['/data/agent-overview/sessions/hook.lock','/data/agent-overview/bin/hash/helper','/data/agent-overview/codex-cache/thread-labels.json','/data/other.json'])
 assert.equal(classify('overview','/data',[p]).snapshot,false,p);
const terminal={address:'t1',state:'live',status:'working',agent:'codex'}, pane={pane:'p1',terminal:'t1'};
assert.equal(nativeKey([], [pane], [terminal]),nativeKey([], [pane], [{...terminal,screen:'new output'}]));
assert.notEqual(nativeKey([], [pane], [terminal]),nativeKey([], [pane], [{...terminal,status:'done'}]));
assert.notEqual(nativeKey([], [pane], [terminal]),nativeKey([], [{...pane,terminal:'t2'}], [terminal]));
const cx={timer:{after:(ms,fn)=>timers.push({ms,fn})},spawn:fn=>fn(cx),notify:()=>{}};
const fire=ms=>{const i=timers.findIndex(t=>t.ms===ms); assert(i>=0,'missing timer '+ms); timers.splice(i,1)[0].fn(cx);};
let refreshes=0, reads=0;
const host={refresh:()=>refreshes++,knownThreads:[a,b,c]};
for(let n=0;n<5;n++) sourceChanged(host,'overview','/data',['/data/agent-overview/sessions/hook.json'],cx);
assert.equal(timers.length,1); fire(180); assert.equal(refreshes,1); assert.equal(timers.length,0);
host.conversation={id:'codex-thread:'+a,data:{watchThreads:[a,b]}};
host.readConversation=()=>reads++;
conversationChanged(host,host.conversation.id,cx); reads=0;
sourceChanged(host,'codex','/codex',[rollout(c)],cx);
conversationChanged(host,host.conversation.id,cx); assert.equal(reads,0,'unrelated transcript');
sourceChanged(host,'codex','/codex',[rollout(b)],cx);
sourceChanged(host,'codex','/codex',[rollout(c)],cx);
conversationChanged(host,host.conversation.id,cx); assert.equal(reads,1,'parent event survives another batch');
conversationChanged(host,host.conversation.id,cx); assert.equal(reads,1,'no render loop');
sourceChanged(host,'codex','/codex',['/codex'],cx);
conversationChanged(host,host.conversation.id,cx); assert.equal(reads,2,'overflow invalidates reader'); fire(180);
assert(values.get(FILE_EVENTS).threads[b]>0);
const snap=codex=>({rows:[],codex});
snapshotReceived(host,snap({refreshing:true,started:false}),cx,host.catalogRevision);
assert.equal(timers.length,0,'wait for worker completion');
snapshotReceived(host,snap({refreshing:false,started:false,refreshAfterMs:25000}),cx,host.catalogRevision);
snapshotReceived(host,snap({refreshing:false,started:false,refreshAfterMs:20000}),cx,host.catalogRevision);
assert.equal(timers.length,1,'one cooldown follow-up'); fire(25000); fire(180);
snapshotReceived(host,snap({refreshing:true,started:true}),cx,host.catalogRevision-1);
assert.equal(host.catalogPending,true,'event during fetch preserved');
snapshotReceived(host,snap({refreshing:true,started:true}),cx,host.catalogRevision);
assert.equal(host.catalogPending,false,'consumed on a new worker');
const watching={rows:[],watch:{overview:'/data',codex:'/codex'}};
watchSources(host,watching,cx); watchSources(host,watching,cx);
assert.equal(watches.get('/data').length,1); assert.equal(watches.get('/codex').length,1);
fire(180); watches.get('/data')[0].resolve(['/data/agent-overview.json']);
await Promise.resolve(); await Promise.resolve();
assert.equal(watches.get('/data').length,2,'watch rearmed'); fire(180);
watches.get('/data')[1].reject(new Error('test watch unavailable'));
await Promise.resolve(); await Promise.resolve();
assert.match(watchError(host),/Kontrollabfrage/); assert.equal(host.watchPaths.overview,null);
watchSources(host,watching,cx); assert.equal(watches.get('/data').length,3,'safety refresh retries watcher');
assert.equal(timers.length,0,'failed watcher retries cannot create a refresh loop');
"#;
    let output = std::process::Command::new("node").args(["--input-type=module", "-e", script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn real_programs_changes_during_snapshot_and_conversation_reads_get_one_followup() {
    let _w = common::world();
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const values=new Map(), timers=[], requests=[];
const id='codex-thread:11111111-1111-1111-1111-111111111111';
values.set('herdr-act.selected',id);
const row={session:{id}}, snapshot={rows:[row],errors:[]}; values.set('herdr-act.snapshot',snapshot);
Object.assign(globalThis,{View:class{},shared:k=>values.get(k),setShared:(k,v)=>values.set(k,v),
 selected:rows=>rows[0],embeddedChild:()=>true,focused:()=>({}),terminals:()=>[],
 liveSnapshot:s=>s,lightSnapshot:s=>s,newQuestions:()=>({projects:[],agents:[],keys:[]}),
 exec:(_binary,args)=>new Promise(resolve=>requests.push({args,resolve})),
});
Object.assign(globalThis,await import(data(fs.readFileSync(path.join(root,'native/act-overview/refresh-events.js'),'utf8').replace(/^import .*;\n/gm,''))));
const {default:Overview}=await import(data(fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'')));
const pending=[], cx={timer:{after:(ms,fn)=>timers.push({ms,fn})},notify:()=>{},spawn:fn=>pending.push(fn(cx))};
const view=new Overview(); view.snapshot=snapshot; view.projectFolded=new Set(); view.childrenFolded=new Set();
view.refresh(cx); view.refresh(cx); view.refresh(cx);
assert.equal(requests.length,1,'serialize snapshots');
requests.shift().resolve({code:0,stdout:JSON.stringify(snapshot)}); await pending.shift();
assert.equal(timers.length,1,'one trailing snapshot'); timers.shift().fn(cx);
assert.equal(requests.length,1); requests.shift().resolve({code:0,stdout:JSON.stringify(snapshot)}); await pending.shift();
assert.equal(timers.length,0,'no idle snapshot loop');
view.readConversation(cx); view.readConversation(cx); view.readConversation(cx);
assert.equal(requests.length,1,'serialize selected reader');
requests.shift().resolve({code:0,stdout:JSON.stringify({id,messages:[{text:'first'}]})}); await pending.shift();
assert.equal(requests.length,1,'event during read causes followup');
requests.shift().resolve({code:0,stdout:JSON.stringify({id,messages:[{text:'latest'}]})}); await pending.shift();
assert.equal(view.conversation.data.messages[0].text,'latest'); assert.equal(requests.length,0);
"#;
    let output = std::process::Command::new("node").args(["--input-type=module", "-e", script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn real_programs_overview_and_conversation_keep_only_a_slow_safety_timer() {
    let _w = common::world();
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
let mode='list';
Object.assign(globalThis,{View:class{},props:()=>({mode}),focused:()=>({tab:'tab',space:'space'}),localStorage:{getItem:()=>null}});
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'');
const {default:Overview}=await import(data(source));
for(mode of ['list','detail']) {
 const intervals=[], view=new Overview(); view.refresh=()=>{};
 view.init({}, {timer:{every:ms=>intervals.push(ms)}});
 assert(!intervals.includes(5000),mode+' must not poll every five seconds');
 assert(intervals.includes(60000),mode+' retains a slow recovery check');
}
"#;
    let output = std::process::Command::new("node").args(["--input-type=module", "-e", script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
