// CLI-level config behavior against a temp dir via HARNESS_NOTIFY_CONFIG_DIR -
// never the operator's real ~/.harness-notify. The suppression case doubles as
// coverage for the `test` subcommand's outcome reporting without firing a real
// OS notification.
use std::path::Path;
use std::process::{Command, Stdio};

fn run_in(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_harness-notify"))
        .args(args)
        .env("HARNESS_NOTIFY_CONFIG_DIR", dir)
        .stdin(Stdio::null())
        .output()
        .expect("binary runs")
}

#[test]
fn set_then_get_round_trips_in_the_overridden_dir() {
    let dir = tempfile::tempdir().unwrap();
    let set = run_in(dir.path(), &["config", "set", "events.subagent_done", "true"]);
    assert!(set.status.success());
    assert!(dir.path().join("config.toml").exists());
    let get = run_in(dir.path(), &["config", "get", "events.subagent_done"]);
    assert!(get.status.success());
    assert_eq!(String::from_utf8_lossy(&get.stdout).trim(), "true");
}

#[test]
fn invalid_set_exits_nonzero_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let set = run_in(dir.path(), &["config", "set", "dnd.start", "10pm"]);
    assert!(!set.status.success());
    assert!(!dir.path().join("config.toml").exists());
}

#[test]
fn test_subcommand_reports_config_suppression_without_firing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(run_in(dir.path(), &["config", "set", "events.done", "false"]).status.success());
    let out = run_in(dir.path(), &["test"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("suppressed by config"));
}
