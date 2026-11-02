//! The Herdr-only Zsh adapter preserves ordinary Codex services and existing shell customizations.
use crate::common;
use sw::agents::overview_events::shell;
use std::{fs,os::unix::fs::PermissionsExt,process::Command};
#[test]
fn real_programs_herdr_shell_keeps_context_without_changing_codex_outside_herdr() {
    let w=common::world();let rc=w.base.join("zshrc");let data=w.base.join("data");let bin=w.base.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(&rc,"# existing settings\nexport KEEP=yes\n").unwrap();
    fs::write(bin.join("codex"),"#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    fs::set_permissions(bin.join("codex"),fs::Permissions::from_mode(0o700)).unwrap();
    assert!(shell::install_at(&rc,&data).unwrap());assert!(!shell::install_at(&rc,&data).unwrap());
    assert!(fs::read_to_string(&rc).unwrap().starts_with("# existing settings\nexport KEEP=yes"));
    let script=data.join("agent-overview/herdr-shell.zsh");
    let path=format!("{}:{}",bin.display(),std::env::var("PATH").unwrap_or_default());
    let run=|managed:bool,expression:&str| {
        let mut c=Command::new("/bin/zsh");c.args(["-f","-c",expression,"_",script.to_str().unwrap()]).env("PATH",&path).env_remove("HC_TERMINAL_ID");
        if managed {c.env("HERDR_ENV","1");} else {c.env_remove("HERDR_ENV");}
        let out=c.output().unwrap();assert!(out.status.success(),"{:?}",out);String::from_utf8(out.stdout).unwrap()
    };
    assert_eq!(run(true,"source \"$1\"; codex --version"),"--no-daemon\n--version\n");
    assert_eq!(run(false,"source \"$1\"; codex --version"),"--version\n");
    assert_eq!(run(false,"export HC_TERMINAL_ID=t7; source \"$1\"; codex --version"),"--no-daemon\n--version\n");
    assert_eq!(run(true,"source \"$1\"; codex app-server --help"),"app-server\n--help\n");
    assert_eq!(run(true,"function codex { print CUSTOM; }; source \"$1\"; codex"),"CUSTOM\n");
}
