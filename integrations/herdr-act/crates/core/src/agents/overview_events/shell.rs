//! Keep genuine Herdr caller context when the app-bundled Codex CLI would otherwise use its shared daemon.
use std::{fs,os::unix::fs::PermissionsExt,path::{Path,PathBuf},process::Command};
const START:&str="# >>> herdr-agent-overview >>>";
const END:&str="# <<< herdr-agent-overview <<<";
pub const SCRIPT:&str=r#"if [[ "${HERDR_ENV:-}" == 1 || -n "${HC_TERMINAL_ID:-}" ]] && (( ! $+functions[codex] && ! $+aliases[codex] )); then
  function codex {
    if [[ "${HERDR_ENV:-}" != 1 && -z "${HC_TERMINAL_ID:-}" ]]; then
      command codex "$@"
      return $?
    fi
    # Daemon-owned services keep their native behavior; terminal conversations retain Herdr's environment.
    case "${1:-}" in
      app|app-server|exec|e|review|login|logout|mcp|plugin|completion|update|doctor|sandbox|debug|apply|a|queue|archive|delete|migrate-rollouts|migrate-nfs-db|unarchive|cloud|exec-server|features|help)
        command codex "$@" ;;
      *) command codex --no-daemon "$@" ;;
    esac
  }
fi
"#;
pub fn install()->Result<Option<PathBuf>,String> {
    if std::env::var("SHELL").ok().and_then(|s|Path::new(&s).file_name().map(|n|n=="zsh"))!=Some(true) {return Ok(None);}
    let supported=Command::new("codex").args(["--no-daemon","--version"]).output().map_err(|e|e.to_string())?;
    if !supported.status.success() {return Err("Codex does not support --no-daemon; no Herdr shell adapter was installed".into());}
    let path=crate::store::files::user_home().join(".zshrc");
    if install_at(&path,&crate::store::files::data_dir())? {Ok(Some(path))} else {Ok(None)}
}
pub fn install_at(zshrc:&Path,data:&Path)->Result<bool,String> {
    let script=data.join("agent-overview/herdr-shell.zsh");
    crate::store::files::write_atomic(&script,SCRIPT.as_bytes()).map_err(|e|e.to_string())?;
    fs::set_permissions(&script,fs::Permissions::from_mode(0o600)).map_err(|e|e.to_string())?;
    let original=match fs::read_to_string(zshrc) {Ok(s)=>s,Err(e)if e.kind()==std::io::ErrorKind::NotFound=>String::new(),Err(e)=>return Err(e.to_string())};
    let mut next=original.clone();
    if let Some(start)=next.find(START) {
        let end=next[start..].find(END).ok_or("Incomplete existing Herdr shell block; left unchanged")?+start+END.len();
        next.replace_range(start..end,"");
    }
    let block=format!("{START}\nif [[ \"${{HERDR_ENV:-}}\" == 1 || -n \"${{HC_TERMINAL_ID:-}}\" ]]; then\n  source {}\nfi\n{END}",crate::util::shq(&script.display().to_string()));
    let next=format!("{}\n\n{block}\n",next.trim_end());
    if original.trim_end()==next.trim_end() {return Ok(false);}
    let mode=fs::metadata(zshrc).map(|m|m.permissions().mode()).unwrap_or(0o600);
    if zshrc.exists() {
        let backup=zshrc.with_extension(format!("before-herdr-overview-{}",super::now_ms()));
        fs::copy(zshrc,&backup).map_err(|e|e.to_string())?;
        fs::set_permissions(backup,fs::Permissions::from_mode(0o600)).map_err(|e|e.to_string())?;
    }
    crate::store::files::write_atomic(zshrc,next.as_bytes()).map_err(|e|e.to_string())?;
    fs::set_permissions(zshrc,fs::Permissions::from_mode(mode)).map_err(|e|e.to_string())?;
    Ok(true)
}
