//! Read daemon-owned runtimes and stored desktop metadata without loading or resuming conversations.
use crate::json::Value;
use serde_json::json;
use std::{collections::BTreeMap, os::unix::net::UnixStream, time::{Duration, Instant}};
use tungstenite::{Message, WebSocket};

const TIMEOUT: Duration = Duration::from_secs(20);
const READ_TIMEOUT: Duration = Duration::from_millis(500);

pub fn list_threads() -> Result<Vec<Value>, String> {
    let path = super::homes::codex_home().join("app-server-control/app-server-control.sock");
    let stream = UnixStream::connect(&path)
        .map_err(|e| format!("Codex Desktop daemon unavailable ({}): {e}", path.display()))?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(|e| e.to_string())?;
    let (socket, _) = tungstenite::client("ws://localhost/", stream)
        .map_err(|e| format!("Codex daemon handshake failed: {e}"))?;
    socket.get_ref().set_read_timeout(Some(READ_TIMEOUT)).map_err(|e| e.to_string())?;
    snapshot(Connection { socket, next_id: 0, requests: Vec::new() })
}

struct Connection {
    socket: WebSocket<UnixStream>,
    next_id: u64,
    requests: Vec<Value>,
}
impl Connection {
    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.socket.send(Message::Text(json!({"method":method,"params":params}).to_string().into()))
            .map_err(|e| e.to_string())
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.socket.send(Message::Text(json!({"id":id,"method":method,"params":params}).to_string().into()))
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if Instant::now() > deadline { return Err(format!("Codex daemon timed out: {method}")); }
            let message = match self.socket.read() {
                Ok(message) => message,
                Err(tungstenite::Error::Io(error)) if matches!(error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::Interrupted) => continue,
                Err(error) => return Err(format!("Codex daemon {method}: {error}")),
            };
            let Message::Text(text) = message else { continue };
            let value: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            if value.get("method").is_some() && value.get("id").is_some() {
                self.requests.push(value);
                continue;
            }
            if value["id"].as_u64() != Some(id) { continue; }
            if let Some(error) = value.get("error") {
                return Err(format!("Codex {method}: {}", error["message"].as_str().unwrap_or("request rejected")));
            }
            return value.get("result").cloned().ok_or_else(|| "Codex returned no result".into());
        }
    }
}
fn snapshot(mut connection: Connection) -> Result<Vec<Value>, String> {
    connection.call("initialize", json!({
        "clientInfo":{"name":"herdr_overview","title":"Herdr Agent Overview","version":"0.1"},
        "capabilities":{"experimentalApi":true}
    }))?;
    connection.notify("initialized", json!({}))?;
    let mut threads = BTreeMap::new();
    let mut cursor = Value::Null;
    for _ in 0..100 {
        let result = connection.call("thread/list", json!({
            "limit":100,"cursor":cursor,"archived":false,
            "sourceKinds":["cli","vscode","appServer","subAgent","subAgentReview","subAgentCompact","subAgentThreadSpawn","subAgentOther"]
        }))?;
        for thread in result["data"].as_array().into_iter().flatten() {
            if let Some(id) = thread["id"].as_str() { threads.insert(id.to_owned(), thread.clone()); }
        }
        cursor = result["nextCursor"].clone();
        if cursor.is_null() { break; }
    }
    // Loaded metadata comes from the live daemon, including threads older than the stored-history page.
    let mut cursor = Value::Null;
    for _ in 0..100 {
        let loaded = connection.call("thread/loaded/list", json!({"limit":100,"cursor":cursor}))?;
        for id in loaded["data"].as_array().into_iter().flatten().filter_map(Value::as_str) {
            if let Ok(result) = connection.call("thread/read", json!({"threadId":id,"includeTurns":false})) {
                if result["thread"].is_object() { threads.insert(id.to_owned(), result["thread"].clone()); }
            }
        }
        cursor = loaded["nextCursor"].clone();
        if cursor.is_null() { break; }
    }
    for request in connection.requests {
        if let Some(thread) = request["params"]["threadId"].as_str().and_then(|id| threads.get_mut(id)) {
            let requests = thread.as_object_mut().unwrap().entry("overviewRequests")
                .or_insert_with(|| json!([])).as_array_mut().unwrap();
            requests.push(request);
        }
    }
    let _ = connection.socket.close(None);
    Ok(threads.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_programs_keeps_waiting_for_a_reply_after_a_socket_read_timeout() {
        let (client, server) = UnixStream::pair().unwrap();
        let worker = std::thread::spawn(move || {
            use std::io::Write;
            let mut socket = tungstenite::accept(server).unwrap();
            let Message::Text(text) = socket.read().unwrap() else { panic!("expected request"); };
            let request: Value = serde_json::from_str(&text).unwrap();
            let reply = json!({"id":request["id"],"result":{"data":[]}}).to_string();
            assert!(reply.len() < 126);
            socket.get_mut().write_all(&[0x81, reply.len() as u8, reply.as_bytes()[0]]).unwrap();
            std::thread::sleep(Duration::from_millis(40));
            socket.get_mut().write_all(&reply.as_bytes()[1..]).unwrap();
        });
        let (socket, _) = tungstenite::client("ws://localhost/", client).unwrap();
        socket.get_ref().set_read_timeout(Some(Duration::from_millis(5))).unwrap();
        let mut connection = Connection { socket, next_id: 0, requests: vec![] };
        let reply = connection.call("thread/list", json!({}));
        worker.join().unwrap();
        assert_eq!(reply.unwrap(), json!({"data":[]}));
    }
    #[test]
    fn real_programs_a_disconnected_daemon_still_reports_the_failed_request() {
        let (client, server) = UnixStream::pair().unwrap();
        let worker = std::thread::spawn(move || {
            let mut socket = tungstenite::accept(server).unwrap();
            socket.read().unwrap();
        });
        let (socket, _) = tungstenite::client("ws://localhost/", client).unwrap();
        let mut connection = Connection { socket, next_id: 0, requests: vec![] };
        assert!(connection.call("thread/list", json!({})).unwrap_err().contains("thread/list"));
        worker.join().unwrap();
    }
    #[test]
    fn observes_loaded_metadata_without_resuming_a_thread() {
        let (client, server) = UnixStream::pair().unwrap();
        let worker = std::thread::spawn(move || {
            let mut socket = tungstenite::accept(server).unwrap();
            let mut methods = Vec::new();
            for _ in 0..5 {
                let Message::Text(text) = socket.read().unwrap() else { panic!("expected JSON"); };
                let value: Value = serde_json::from_str(&text).unwrap();
                let method = value["method"].as_str().unwrap();
                methods.push(method.to_owned());
                let Some(id) = value.get("id") else { continue };
                let result = match method {
                    "initialize" => json!({}),
                    "thread/list" => json!({"data":[{"id":"desktop","status":{"type":"notLoaded"}}]}),
                    "thread/loaded/list" => json!({"data":["desktop"]}),
                    "thread/read" => json!({"thread":{"id":"desktop","status":{"type":"active","activeFlags":["waitingOnApproval"]}}}),
                    _ => panic!("unexpected mutation: {method}"),
                };
                socket.send(Message::Text(json!({"id":id,"result":result}).to_string().into())).unwrap();
            }
            methods
        });
        let (socket, _) = tungstenite::client("ws://localhost/", client).unwrap();
        let threads = snapshot(Connection { socket, next_id: 0, requests: vec![] }).unwrap();
        assert_eq!(threads[0]["status"]["type"], "active");
        assert!(!worker.join().unwrap().iter().any(|m| m.contains("resume") || m.contains("start")));
    }
}
