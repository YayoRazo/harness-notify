// Exit-code contract: interactive subcommands (install/uninstall/config)
// exit non-zero on a runtime failure so a script or agent driving them can
// detect it; notify keeps its unconditional exit-0 guarantee because hooks
// call it unattended. No case here writes anything: the failing ones error
// out before any write and the passing ones only read, so no real user
// config is touched.
use std::process::{Command, Stdio};

fn run(args: &[&str]) -> std::process::ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_harness-notify"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("binary runs")
}

#[test]
fn install_with_unknown_harness_exits_nonzero() {
    assert!(!run(&["install", "--harness", "not-a-real-harness"]).success());
}

#[test]
fn install_on_a_config_command_only_harness_exits_nonzero() {
    assert!(!run(&["install", "--harness", "kilo"]).success());
}

#[test]
fn config_get_with_unknown_key_exits_nonzero() {
    assert!(!run(&["config", "get", "nope.nope"]).success());
}

#[test]
fn config_get_with_known_key_exits_zero() {
    assert!(run(&["config", "get", "events.done"]).success());
}

#[test]
fn notify_keeps_its_exit_zero_guarantee_even_on_a_bogus_event() {
    assert!(run(&["notify", "--event", "bogus"]).success());
}
