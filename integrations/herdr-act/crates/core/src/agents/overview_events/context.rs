//! Resolve ACT's configured ticket backend and provide the agent's explicit reporting contract.
use std::{fs, io::Read, path::{Path, PathBuf}};
use regex::Regex;
use yaml_rust2::YamlLoader;

#[derive(Default)]
pub struct Context { pub project: String, pub ticket: Option<String> }
fn read(path: &Path) -> Option<String> {
    let mut text = String::new();
    fs::File::open(path).ok()?.take(128_000).read_to_string(&mut text).ok()?;
    Some(text)
}
pub fn resolve(cwd: &str, words: &str) -> Context {
    let cwd = Path::new(cwd);
    if !cwd.is_absolute() { return Context::default(); }
    let root = cwd.ancestors().find(|p| p.join(".git").exists() || p.join(".act/config.yaml").exists()).unwrap_or(cwd);
    let common = read(&root.join(".git")).and_then(|s| s.strip_prefix("gitdir: ").map(str::trim).map(str::to_owned))
        .and_then(|s| root.join(s).canonicalize().ok())
        .and_then(|p| p.ancestors().find(|a| a.file_name().is_some_and(|n| n == ".git")).and_then(Path::parent).map(Path::to_path_buf))
        .unwrap_or_else(|| root.to_path_buf());
    let project = common.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let config = [root.join(".act/config.yaml"), common.join(".act/config.yaml")].into_iter()
        .find_map(|p| read(&p)).and_then(|s| YamlLoader::load_from_str(&s).ok()).and_then(|mut v| if v.is_empty() { None } else { Some(v.remove(0)) });
    let backend = config.as_ref().and_then(|v| v["workflow"]["backend"].as_str()).unwrap_or("");
    let github = Regex::new(r"https://github\.com/[\w.-]+/[\w.-]+/(?:issues|pull)/[0-9]+|[\w.-]+/[\w.-]+#[0-9]+").unwrap();
    let mut ticket = github.find(words).map(|m| m.as_str().to_owned());
    if ticket.is_none() && backend == "github" {
        if let Some(number) = Regex::new(r"(?:^|\s)#([0-9]+)\b").unwrap().captures(words) {
            let remote = read(&common.join(".git/config")).unwrap_or_default();
            let repo = Regex::new(r"github\.com[:/]([\w.-]+/[\w.-]+?)(?:\.git)?(?:\s|$)").unwrap()
                .captures(&remote).map(|c| c[1].to_owned()).unwrap_or_default();
            ticket = Some(format!("{repo}#{}", &number[1]));
        }
    }
    if ticket.is_none() && backend == "local" {
        if let Some(path) = config.as_ref().and_then(|v| v["local"]["path"].as_str()) {
            let base = root.join(path);
            if let Ok(allowed) = base.canonicalize() {
                for token in words.split_whitespace() {
                    let token = token.trim_matches(['\u{60}', '\'', '"', '(', ')', ',', ';']);
                    if !token.ends_with(".md") { continue; }
                    for candidate in [root.join(token), base.join(token)] {
                        let Ok(candidate) = candidate.canonicalize() else { continue };
                        if !candidate.starts_with(&allowed) { continue; }
                        let title = read(&candidate).and_then(|s| s.lines().find_map(|l| l.strip_prefix("# ").map(str::to_owned)));
                        ticket = Some(match title { Some(t) => format!("{} · {t}", candidate.display()), None => candidate.display().to_string() });
                        break;
                    }
                    if ticket.is_some() { break; }
                }
            }
        }
    }
    Context { project, ticket }
}
pub fn instructions(provider: &str, session: &str) -> Result<String, String> {
    let binary: PathBuf = std::env::current_exe().map_err(|e| e.to_string())?;
    let quote = crate::util::shq;
    let command = format!("env {} {} agents report --source {} --session {}",
        quote(&format!("HERDR_PROJECTS_DATA_DIR={}", crate::store::files::data_dir().display())),
        quote(&binary.display().to_string()), quote(provider), quote(session));
    Ok(format!(
        "Herdr Agent Overview telemetry (local only): explicit ACT slash/skill invocations are observed automatically. \
         When you enter an ACT skill automatically without a slash command or Skill tool, run {command} --phase COMMAND once at entry; \
         COMMAND is act-interview, act-create-spec, act-refine-spec, act-create-issues, act-implement (Flutter variants supported), or land-feature. \
         After that phase has actually finished run {command} --phase COMMAND --phase-state end; phases may repeat or be skipped. \
         Add --ticket REF when a Spec/ticket is known. When asking a concrete user question in final prose instead of a structured input tool, \
         report it with {command} --question TEXT before yielding. Only after the entire assigned task and its acceptance criteria are complete, \
         run {command} --state done. A stopped turn, an answered interview, a phase end, or waiting for approval does not complete the assignment. \
         Do not report done while required work or verification remains. These commands never approve tools or answer questions."
    ))
}
