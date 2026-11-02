//! [real programs] The page server's front door, what the first port left out: turning the board in the web UI off
//! points its link back and closes the door (its bookkeeping says so), on again it is back on the port it had, and it
//! closes cleanly. The door's loop takes its looks here one at a time (Tailscale is a stand-in program).
use crate::common;
use common::*;
use serde_json::json;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use sw::store::settings::{change::{set_value, unset_value}, lookup::Scope};
use sw::webui::door_loop::DoorLoop;
use sw::webui::keep::{door_file, door_info};

const SCRIPT: &str = r#"#!/bin/sh
D='@DIR@'
echo "$*" >> "$D/ran"
if [ "$*" = "serve status --json" ]; then
  tcp=""; web=""
  for f in "$D"/web/*; do [ -e "$f" ] || continue; p=$(basename "$f"); t=$(cat "$f")
    tcp="$tcp${tcp:+,}\"$p\":{\"HTTPS\":true}"
    web="$web${web:+,}\"box.tail1.ts.net:$p\":{\"Handlers\":{\"/\":{\"Proxy\":\"$t\"}}}"
  done
  printf '{"TCP":{%s},"Web":{%s},"AllowFunnel":{}}' "$tcp" "$web"; exit 0
fi
if [ "$1" = serve ] && [ "$2" = --bg ]; then p="${3#--https=}"; printf %s "$4" > "$D/web/$p"; exit 0; fi
echo "unexpected: tailscale $*" >&2; exit 1
"#;
/// A stand-in for `tailscale serve`, first on the PATH: where each private link goes is a file.
fn tailnet(base: &Path) -> PathBuf {
    let dir = base.join("tailscale");
    for d in ["web", "bin"] { std::fs::create_dir_all(dir.join(d)).unwrap(); }
    let script = dir.join("bin/tailscale");
    std::fs::write(&script, SCRIPT.replace("@DIR@", &dir.display().to_string())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = std::env::var("PATH").unwrap_or_default();
    let path = path.split(':').filter(|p| !p.ends_with("tailscale/bin")).collect::<Vec<_>>().join(":");
    unsafe { std::env::set_var("PATH", format!("{}:{path}", dir.join("bin").display())) };
    dir
}
/// A stand-in web UI that says it is one.
fn web_ui() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || for c in l.incoming() { let Ok(mut c) = c else { continue }; std::thread::spawn(move || {
        let (head, _) = sw::webui::proxy::read_head(&mut c, 1 << 20).unwrap_or_default();
        let health = String::from_utf8_lossy(&head).starts_with("GET /api/health");
        let (kind, body) = if health { ("application/json", r#"{"ok":true,"auth":{"required":true,"authenticated":false}}"#) } else { ("text/html", "<html><body>web ui</body></html>") };
        let _ = write!(c, "HTTP/1.1 200 OK\r\ncontent-type: {kind}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
    }); });
    port
}

#[test]
fn real_programs_the_front_door_closes_when_the_board_in_the_web_ui_is_turned_off_its_link_goes_back_on_again_it_is_on_its_port_and_it_closes_cleanly() {
    let w = world();
    sw::extras::note(json!({ "webUi": "", "webUiLinks": [] }));
    let ts = tailnet(&w.base);
    let at = |https: u16| read(&ts.join("web").join(https.to_string()));
    let ui = web_ui();
    unsafe { std::env::set_var("HERDR_PROJECTS_WEBUI", ui.to_string()) };
    // the port it had (a free one: the first one it would ask for may be a real board's)
    let free = { let l = TcpListener::bind("127.0.0.1:0").unwrap(); l.local_addr().unwrap().port() };
    sw::store::files::write_json(&door_file(), &json!({ "port": free })).unwrap();
    let mut door = DoorLoop::new();
    door.tick();
    assert_eq!(door.port(), Some(free));
    assert_eq!(door_info().map(|d| d["port"].clone()), Some(json!(free)));
    // a link that was given to it and still points at it
    std::fs::write(ts.join("web/443"), format!("http://127.0.0.1:{ui}")).unwrap();
    sw::webui::link::link(ui, free);
    assert_eq!(at(443), format!("http://127.0.0.1:{free}"));
    // the board in the web UI is turned off: the link goes back before the door closes, and its bookkeeping says so
    set_value("board.webUi", &json!("none"), &Scope::you()).unwrap();
    door.tick();
    assert_eq!(door.port(), None);
    assert_eq!(sw::store::files::read_json(&door_file()), Some(json!({})));
    assert!(door_info().is_none());
    assert_eq!(at(443), format!("http://127.0.0.1:{ui}"));
    // on again: it is back on the port it had, so a link that points at it still does
    unset_value("board.webUi", &Scope::you()).unwrap();
    door.tick();
    assert_eq!(door.port(), Some(free));
    assert!(TcpStream::connect(("127.0.0.1", free)).is_ok());
    // closed cleanly
    door.stop();
    assert_eq!(door.port(), None);
    assert_eq!(sw::store::files::read_json(&door_file()), Some(json!({})));
    unsafe { std::env::remove_var("HERDR_PROJECTS_WEBUI") };
}
