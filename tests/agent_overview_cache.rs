//! Native refreshes keep working while Codex's slower history refresh is still waiting for the daemon.
use crate::common::*;
use serde_json::Value;
use std::{fs, os::unix::{fs::PermissionsExt, net::UnixListener}, process::Command};
use sw::agents::codex_app_cache as cache;

#[test]
fn cached_threads_stay_available_during_refresh_and_after_a_failure() {
    let w = world();
    let directory = w.base.join("cache");
    let old = vec![serde_json::json!({"id":"previous"})];
    cache::refresh_at(&directory, || 0, || Ok(old.clone())).unwrap();
    cache::refresh_at(&directory, || 30_000, || {
        let during = cache::read_at(&directory, 30_000, true).unwrap();
        assert_eq!(during.threads, old);
        assert!(during.refreshing && !during.refresh_due);
        Err("Codex disconnected".into())
    }).unwrap();
    let failed = cache::read_at(&directory, 35_000, false).unwrap();
    assert_eq!(failed.threads, old);
    assert_eq!(failed.updated_at, Some(0));
    assert_eq!(failed.error.as_deref(), Some("Codex disconnected"));
    assert!(!failed.refresh_due);
    cache::refresh_at(&directory, || 60_000, || Ok(vec![])).unwrap();
    let recovered = cache::read_at(&directory, 60_000, false).unwrap();
    assert!(recovered.threads.is_empty() && recovered.error.is_none());
    assert_eq!(recovered.updated_at, Some(60_000));
}

#[test]
fn only_one_worker_runs_and_successful_history_is_not_fetched_on_every_poll() {
    let w = world();
    let directory = w.base.join("cache");
    let cold = cache::read_at(&directory, 0, false).unwrap();
    assert!(cold.threads.is_empty() && cold.refresh_due);
    let held = sw::store::files::hold_lock(cache::LOCK, 0).unwrap();
    assert!(!cache::refresh_at(&directory, || 0, || panic!("another worker owns the refresh")).unwrap());
    drop(held);
    cache::refresh_at(&directory, || 0, || Ok(vec![])).unwrap();
    assert!(!cache::refresh_at(&directory, || 5_000, || panic!("cache is still fresh")).unwrap());
    assert!(cache::read_at(&directory, 30_000, false).unwrap().refresh_due);
}

#[test]
fn refresh_interval_starts_after_a_slow_fetch_and_metadata_stays_private() {
    let w = world();
    let directory = w.base.join("cache");
    let clock = std::cell::Cell::new(0);
    cache::refresh_at(&directory, || clock.get(), || { clock.set(90_000); Ok(vec![]) }).unwrap();
    let fresh = cache::read_at(&directory, 95_000, false).unwrap();
    assert_eq!(fresh.updated_at, Some(90_000));
    assert!(!fresh.refresh_due);
    assert!(cache::read_at(&directory, 120_000, false).unwrap().refresh_due);
    assert_eq!(fs::metadata(&directory).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(directory.join("snapshot.json")).unwrap().permissions().mode() & 0o777, 0o600);
}

#[test]
fn real_programs_native_snapshot_does_not_wait_for_an_inflight_codex_history_refresh() {
    let w = world();
    let bin = w.base.join("commands");
    fs::create_dir_all(&bin).unwrap();
    for (name, json) in [("herdr-app", r#"{"spaces":[],"tabs":[]}"#), ("herdr-core", r#"{"terminals":[]}"#)] {
        let program = bin.join(name);
        fs::write(&program, format!("#!/bin/sh\nprintf '%s\\n' '{json}'\n")).unwrap();
        fs::set_permissions(program, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let control = w.base.join("codex/app-server-control");
    fs::create_dir_all(&control).unwrap();
    let _unresponsive_daemon = UnixListener::bind(control.join("app-server-control.sock")).unwrap();
    let _refresh = sw::store::files::hold_lock("codex-overview-refresh", 0).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_herdr-projects"))
        .args(["agents", "native-snapshot"])
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
        .output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let snapshot: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(snapshot["errors"], serde_json::json!([]), "{snapshot}");
    assert_eq!(snapshot["codex"]["refreshing"], true);
}
