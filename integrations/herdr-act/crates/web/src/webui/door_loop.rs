//! The front door's loop: in the page server's process, it opens the front door while the web UI plugin is installed
//! (find.rs), follows the web UI to another port, hands over to a new process when the program was updated, and stops
//! cleanly. Whether it is up, and starting it from outside: keep.rs.
use super::door::{handle, Upstream};
use super::find::{web_ui, WebUi};
use super::keep::{door_file, off, FIRST_PORT};
use super::link;
use crate::json::*;
use crate::store::files::{marker, read_json, write_json, VERSION};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const EVERY: u64 = 30;
/// What the requests are passed on to, as the door's loop last saw it.
#[derive(Default)]
struct Door { to: Mutex<Option<WebUi>>, linked: Mutex<String>, looked_at: Mutex<i64> }
impl Door {
    fn find(&self) -> Option<WebUi> { *self.looked_at.lock().unwrap() = crate::clock::now(); let w = web_ui(true); *self.to.lock().unwrap() = w.clone(); w }
}
impl Upstream for Door {
    fn to(&self) -> Option<(String, u16)> { self.to.lock().unwrap().as_ref().map(|w| (w.host.clone(), w.port)) }
    fn link(&self) -> String { self.linked.lock().unwrap().clone() }
    /// The web UI did not answer: it may have moved to another port.
    fn down(&self) { if crate::clock::now() - *self.looked_at.lock().unwrap() > 5000 { self.find(); } }
}
/// A listener and the switch that stops its loop.
struct Open { port: u16, on: Arc<AtomicBool> }
/// The port it had may still be held for a moment by the process it takes over from; then the first one; then any.
fn listen(wants: &[(u16, u32)]) -> Option<TcpListener> {
    for (port, tries) in wants {
        for n in 0..*tries { if let Ok(l) = TcpListener::bind(("127.0.0.1", *port)) { return Some(l); } if n + 1 < *tries { std::thread::sleep(std::time::Duration::from_millis(250)); } }
    }
    None
}
fn open(door: &Arc<Door>, wants: &[(u16, u32)]) -> Option<Open> {
    let l = listen(wants)?;
    let port = l.local_addr().ok()?.port();
    let on = Arc::new(AtomicBool::new(true));
    let (d, flag) = (door.clone(), on.clone());
    std::thread::spawn(move || {
        for conn in l.incoming() {
            if !flag.load(Ordering::SeqCst) { break; }
            let Ok(c) = conn else { continue };
            let d = d.clone();
            std::thread::spawn(move || handle(c, &*d));
        }
    });
    Some(Open { port, on })
}
fn shut(o: &Open) { o.on.store(false, Ordering::SeqCst); let _ = TcpStream::connect(("127.0.0.1", o.port)); }
/// The front door's loop, one look at a time: what it has open, and the port it had (a test takes the looks itself).
pub struct DoorLoop { door: Arc<Door>, server: Option<Open>, saved: u16 }
impl Default for DoorLoop { fn default() -> Self { Self::new() } }
impl DoorLoop {
    pub fn new() -> Self { DoorLoop { door: Arc::new(Door::default()), server: None, saved: read_json(&door_file()).unwrap_or(Value::Null).i("port") as u16 } }
    /// The port it listens on, while it is open.
    pub fn port(&self) -> Option<u16> { self.server.as_ref().map(|o| o.port) }
    fn write(port: u16, to: &Option<WebUi>) { let mut m = marker(); m.put("port", port); m.put("to", to.as_ref().map(|w| json!({ "host": w.host, "port": w.port })).unwrap_or(Value::Null)); m.put("version", VERSION); let _ = write_json(&door_file(), &Value::Object(m)); }
    /// One look: open while there is a web UI, closed (its link pointed back) while there is none.
    pub fn tick(&mut self) {
        let before = self.door.to.lock().unwrap().clone();
        let to = self.door.find();
        match (&to, &self.server) {
            // The web UI is gone, or the board in it was turned off: its link goes back. On again, it asks for this port.
            (None, Some(o)) => { let _ = link::unlink(Some(o.port), before.as_ref().map(|w| w.port)); self.saved = o.port; shut(o); self.server = None; *self.door.linked.lock().unwrap() = String::new(); let _ = write_json(&door_file(), &json!({})); }
            (None, None) => {}
            (Some(w), None) => {
                // The board shows the herdr the web UI shows (find.rs says which).
                if !w.socket.is_empty() { unsafe { std::env::set_var("HERDR_SOCKET_PATH", &w.socket); } }
                let mut wants = vec![];
                if self.saved > 0 { wants.push((self.saved, 12)); }
                wants.push((FIRST_PORT, 1)); wants.push((0, 1));
                if let Some(o) = open(&self.door, &wants) { Self::write(o.port, &to); self.server = Some(o); }
            }
            (Some(w), Some(o)) => if before.as_ref().map(|b| (b.port, &b.host)) != Some((w.port, &w.host)) { Self::write(o.port, &to); },
        }
        // Its links are where it is, and go back to where the web UI is now: one that still points at where the
        // front door was is brought along. A link made for the web UI later is left alone.
        if let (Some(w), Some(o)) = (&to, &self.server) {
            link::follow(o.port, (self.saved > 0 && self.saved != o.port).then_some(self.saved), Some(w.port));
            *self.door.linked.lock().unwrap() = link::look(w.port, Some(o.port)).ours.iter().find(|e| !e.funnel).map(|e| e.url.clone()).unwrap_or_default();
        }
    }
    /// Closed cleanly: nothing listens, and its bookkeeping says so.
    pub fn stop(&mut self) { if let Some(o) = self.server.take() { shut(&o); } let _ = write_json(&door_file(), &json!({})); }
}
/// In the page server's process: opens the front door while there is a web UI, and keeps looking. Never fails.
pub fn run_door() {
    if off() { return; }
    let exe = crate::store::files::bin();
    let stamp = crate::keeper::code_stamp(&exe);
    let mut d = DoorLoop::new();
    std::thread::spawn(move || loop {
        // The program was updated: a new process takes over, on the same port (the file keeps saying which).
        if crate::keeper::code_stamp(&exe) != stamp {
            if let Some(o) = d.server.take() { shut(&o); }
            crate::housekeeping::spawn_loose(&["serve"]);
            std::thread::sleep(std::time::Duration::from_millis(50));
            std::process::exit(0);
        }
        d.tick();
        std::thread::sleep(std::time::Duration::from_secs(EVERY));
    });
}
