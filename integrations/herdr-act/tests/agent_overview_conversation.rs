//! Embedded Codex children open their own saved conversation instead of focusing the shared parent terminal.
use crate::common;
use serde_json::{json, Value};
use std::{fs, path::Path};
use sw::agents::{overview, overview_conversation};

const CHILD: &str = "11111111-1111-4111-8111-111111111111";
const PARENT: &str = "22222222-2222-4222-8222-222222222222";
fn saved(home: &Path, id: &str, rows: &[Value]) -> std::path::PathBuf {
    let directory = home.join("codex/sessions");
    fs::create_dir_all(&directory).unwrap();
    let file = directory.join(format!("rollout-{id}.jsonl"));
    fs::write(&file, rows.iter().map(Value::to_string).collect::<Vec<_>>().join("\n") + "\n").unwrap();
    file
}
fn child_meta() -> Value {
    json!({"type":"session_meta","payload":{"id":CHILD,"cwd":"/repo","source":{"subagent":{"thread_spawn":{
        "parent_thread_id":PARENT,"agent_path":"/root/code_reviewer","agent_nickname":"Ada"
    }}}}})
}
#[test]
fn conversation_reader_shows_the_delegated_task_and_visible_messages_without_mutating_files() {
    let w = common::world();
    let parent = saved(&w.base, PARENT, &[
        json!({"type":"response_item","payload":{"type":"function_call","name":"spawn_agent","call_id":"spawn","arguments":json!({"task_name":"code_reviewer","message":"Check the photo viewer and large text."}).to_string()}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"spawn","output":json!({"task_name":"/root/code_reviewer"}).to_string()}}),
    ]);
    let file = saved(&w.base, CHILD, &[
        child_meta(),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"# AGENTS.md instructions for /repo"}]}}),
        json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"Private reasoning"}]}}),
        json!({"type":"response_item","payload":{"type":"message","role":"assistant","channel":"analysis","content":[{"type":"output_text","text":"Private analysis"}]}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","output":"Private raw tool output"}}),
        json!({"type":"response_item","timestamp":"2026-11-02T12:00:00Z","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"The viewer is ready."}]}}),
    ]);
    let before = (fs::read(&file).unwrap(), fs::read(&parent).unwrap());
    let result = overview_conversation::read(&format!("codex-thread:{CHILD}")).unwrap();
    assert_eq!(result["title"], "code reviewer");
    assert_eq!(result["watchThreads"], json!([CHILD, PARENT]));
    assert_eq!(result["assignment"], "Check the photo viewer and large text.");
    assert_eq!(result["messages"].as_array().unwrap().len(), 1);
    assert_eq!(result["messages"][0]["text"], "The viewer is ready.");
    assert_eq!((fs::read(file).unwrap(), fs::read(parent).unwrap()), before);
}
#[test]
fn conversation_reader_has_honest_missing_states_and_rejects_path_traversal_and_wrong_identities() {
    let w = common::world();
    assert!(overview_conversation::read("codex-thread:../../private").is_err());
    assert_eq!(overview_conversation::read(&format!("codex-thread:{CHILD}")).unwrap()["available"], false);
    let mut meta = child_meta(); meta["payload"]["id"] = json!(PARENT);
    saved(&w.base, CHILD, &[meta]);
    assert!(overview_conversation::read(&format!("codex-thread:{CHILD}")).is_err());
}
#[test]
fn conversation_reader_labels_children_from_saved_roles_and_preserves_user_titles() {
    let w = common::world();
    saved(&w.base, CHILD, &[child_meta()]);
    let mut sessions = overview::from_herdr(&[json!({"agent":"codex","pane_id":"t1","name":"default"})], &[]);
    sessions[0].id = format!("codex-thread:{CHILD}");
    sessions[0].title = "default".into();
    sessions[0].parent_id = Some(format!("codex-thread:{PARENT}"));
    overview_conversation::label_children(&mut sessions).unwrap();
    assert_eq!(sessions[0].title, "code reviewer");
    sessions[0].title = "My reviewer".into();
    overview_conversation::label_children(&mut sessions).unwrap();
    assert_eq!(sessions[0].title, "My reviewer");
}

#[test]
fn conversation_reader_bounds_long_history_without_splitting_unicode() {
    let w = common::world();
    let mut rows = vec![child_meta()];
    for n in 0..84 {
        rows.push(json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":format!("Update {n}")}]}}));
    }
    rows.push(json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"🙂".repeat(33_000)}]}}));
    saved(&w.base, CHILD, &rows);
    let data = overview_conversation::read(&format!("codex-thread:{CHILD}")).unwrap();
    let messages = data["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 80);
    assert_eq!(messages.last().unwrap()["text"].as_str().unwrap().chars().count(), 32_000);
    assert_eq!(data["truncated"], true);
}

#[test]
fn conversation_reader_omits_sealed_inputs_instead_of_showing_ciphertext_as_a_task() {
    let w = common::world();
    let sealed = format!("gAAAAA{}==", "a".repeat(160));
    saved(&w.base, PARENT, &[
        json!({"type":"response_item","payload":{"type":"function_call","name":"spawn_agent","call_id":"spawn","arguments":json!({"message":sealed}).to_string()}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"spawn","output":json!({"task_name":"/root/code_reviewer"}).to_string()}}),
    ]);
    saved(&w.base, CHILD, &[
        child_meta(),
        json!({"type":"event_msg","payload":{"type":"user_message","message":sealed}}),
        json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Visible review result"}]}}),
    ]);
    let result = overview_conversation::read(&format!("codex-thread:{CHILD}")).unwrap();
    assert!(result["assignment"].is_null());
    assert_eq!(result["messages"].as_array().unwrap().len(), 1);
    assert!(!result.to_string().contains(&sealed));
}

#[test]
fn real_programs_conversation_refresh_ignores_late_replies_and_retains_the_last_good_view() {
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const parent={id:'codex-thread:parent',source:{Native:{terminal:'t1'}}};
const rows=[{session:parent},...['a','b','c'].map(id=>({session:{id:'codex-thread:'+id,parent_id:parent.id,source:{Native:{terminal:'t1'}}}}))];
const values=new Map([['herdr-act.snapshot',{rows}]]), requests=[], pending=[];
Object.assign(globalThis,model,{View:class{},shared:k=>values.get(k),focused:()=>({terminal:'t1'}),
exec:(_bin,args)=>new Promise((resolve,reject)=>requests.push({id:args[2],resolve,reject}))});
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'').replace(/^/,'const {CHOSEN,chooseProject,followFocus,projectDetail}={CHOSEN:"herdr-act.chosen-project",chooseProject(){},followFocus(){},projectDetail(){}};\n');
const {default:Overview}=await import(data(source)), view=Object.create(Overview.prototype);
const cx={notify(){},spawn(fn){const task=fn(cx);pending.push(task);return task;}};
const load=id=>{values.set('herdr-act.selected','codex-thread:'+id);view.readConversation(cx);return pending.pop();};
const reply=id=>({code:0,stdout:JSON.stringify({id:'codex-thread:'+id,available:true,messages:[]})});
const a=load('a'), b=load('b');
requests[1].resolve(reply('b')); await b;
requests[0].resolve(reply('a')); await a;
assert.equal(view.conversation.id,'codex-thread:b'); assert.equal(view.conversation.data.id,'codex-thread:b');
view.conversation.assignmentExpanded=true;
const retry=load('b'); requests[2].reject(new Error('offline')); await retry;
assert.equal(view.conversation.data.id,'codex-thread:b'); assert.equal(view.conversation.error,'offline');
assert.equal(view.conversation.assignmentExpanded,true,'Refreshing keeps an expanded task');
const task=load('c'); assert.equal(view.conversation.assignmentExpanded,false);
values.set('herdr-act.selected',parent.id); view.readConversation(cx);
requests[3].resolve(reply('c')); await task;
assert.equal(view.conversation,null,'Returning to the parent invalidates pending child replies');
"#;
    let output = std::process::Command::new("node").args(["--input-type=module", "-e", script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn real_programs_a_codex_child_opens_its_conversation_and_can_return_to_its_parent() {
    let _w = common::world();
    let script = r#"import fs from 'node:fs'; import path from 'node:path'; import assert from 'node:assert/strict';
const root=process.argv[1], data=s=>'data:text/javascript;base64,'+Buffer.from(s).toString('base64');
const model=await import(data(fs.readFileSync(path.join(root,'native/act-overview/model.js'),'utf8')));
const parent={id:'codex-thread:parent',program:'codex',source:{Native:{terminal:'t1'}}};
const child={id:'codex-thread:child',program:'codex',parent_id:parent.id,source:{Native:{terminal:'t1'}}};
const rows=[{session:parent},{session:child}], values=new Map(), commands=[];
Object.assign(globalThis,model,{View:class{},run:(...a)=>commands.push(a),
 shared:k=>values.get(k),setShared:(k,v)=>values.set(k,v),focused:()=>({terminal:'t1'}),
 terminals:()=>[{address:'t1',state:'live'}],panes:()=>[{pane:'p1',terminal:'t1'}]});
const source=fs.readFileSync(path.join(root,'native/act-overview/main.js'),'utf8').replace(/^import .*;\n/gm,'').replace(/^/,'const {CHOSEN,chooseProject,followFocus,projectDetail}={CHOSEN:"herdr-act.chosen-project",chooseProject(){},followFocus(){},projectDetail(){}};\n');
const ActOverview=(await import(data(source))).default, view=Object.create(ActOverview.prototype);
view.snapshot={rows,errors:[]};
view.choose(rows[1],{});
assert(commands.some(([name,args])=>name==='layout.hide'&&args.id==='act-terminal'),'A shared-terminal child needs its own conversation view');
assert(!commands.some(([name])=>name==='pane.focus'||name==='terminal.peek'),'Do not refocus the parent for a child');
assert.equal(model.selected(rows,child.id,null)?.session.id,child.id,'Keep the child selected while the terminal is hidden');
assert.equal(model.selected(rows,child.id,'another-space')?.session.id,child.id,'All Spaces can inspect a child without focusing its shared terminal');
commands.length=0;
view.mode='detail'; view.snapshot={rows:[],errors:[]};
view.choose(rows[0],{});
assert(commands.some(([name,args])=>name==='layout.show'&&args.id==='act-terminal'));
assert(commands.some(([name,args])=>name==='pane.focus'&&args.pane==='p1'));
assert.equal(values.get('herdr-act.snapshot').rows.length,2,'Returning from the detail pane keeps the shared session metadata');
"#;
    let output = std::process::Command::new("node").args(["--input-type=module", "-e", script]).arg(env!("CARGO_MANIFEST_DIR")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}
