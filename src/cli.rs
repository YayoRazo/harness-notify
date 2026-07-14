use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "harness-notify", version, about = "Cross-platform, multi-harness desktop notifier for AI coding agents.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Notify {
        #[arg(long)]
        harness: Option<String>,
        #[arg(long)]
        event: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        message: Option<String>,
        /// The calling project's directory, for harnesses (like opencode)
        /// that don't pipe a JSON payload with a `cwd` field on stdin.
        /// Ignored when stdin already supplies one.
        #[arg(long)]
        cwd: Option<String>,
    },
    Install {
        #[arg(long)]
        harness: String,
        #[arg(long)]
        project: bool,
    },
    Uninstall {
        #[arg(long)]
        harness: String,
        #[arg(long)]
        project: bool,
    },
    Test {
        #[arg(long)]
        harness: Option<String>,
    },
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Checks whether the OS will actually display a notification, and
    /// prints a warning if it looks disabled. Called from a session-start
    /// hook, not by the operator directly - always exits 0. `--hook` is
    /// informational (which hook point called it) and also gives adapters
    /// a distinctive, multi-word command string to use as their own-entry
    /// marker, instead of a single generic word.
    Check {
        #[arg(long)]
        hook: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    Get { key: String },
    Set { key: String, value: String },
    List,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_notify_with_all_flags() {
        let cli = Cli::try_parse_from([
            "harness-notify", "notify", "--harness", "claude-code", "--event", "done", "--message", "hi",
        ]).unwrap();
        match cli.command {
            Some(Command::Notify { harness, event, message, .. }) => {
                assert_eq!(harness.as_deref(), Some("claude-code"));
                assert_eq!(event, "done");
                assert_eq!(message.as_deref(), Some("hi"));
            }
            _ => panic!("expected Notify"),
        }
    }

    #[test]
    fn notify_never_hard_fails_on_an_unknown_event_string() {
        // clap parses --event as a plain String (not an enum) precisely so
        // an unrecognized value is a runtime no-op, not a parse error that
        // would make the process exit non-zero before dispatch even runs.
        let cli = Cli::try_parse_from([
            "harness-notify", "notify", "--event", "bogus",
        ]).unwrap();
        assert!(matches!(cli.command, Some(Command::Notify { .. })));
    }

    #[test]
    fn parses_install_with_project_flag() {
        let cli = Cli::try_parse_from([
            "harness-notify", "install", "--harness", "opencode", "--project",
        ]).unwrap();
        match cli.command {
            Some(Command::Install { harness, project }) => {
                assert_eq!(harness, "opencode");
                assert!(project);
            }
            _ => panic!("expected Install"),
        }
    }

    #[test]
    fn notify_missing_required_event_flag_is_a_clap_parse_error() {
        // Proves the precondition for main.rs's malformed-notify handling:
        // clap DOES reject this as Err (missing required --event), so the
        // exit(0)-vs-e.exit() branch in main() actually has work to do.
        let result = Cli::try_parse_from([
            "harness-notify", "notify", "--harness", "claude-code",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn no_subcommand_parses_to_none() {
        let cli = Cli::try_parse_from(["harness-notify"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_check() {
        let cli = Cli::try_parse_from(["harness-notify", "check", "--hook", "session-start"]).unwrap();
        match cli.command {
            Some(Command::Check { hook }) => assert_eq!(hook.as_deref(), Some("session-start")),
            _ => panic!("expected Check"),
        }
    }

    #[test]
    fn parses_config_set() {
        let cli = Cli::try_parse_from([
            "harness-notify", "config", "set", "sound.enabled", "false",
        ]).unwrap();
        match cli.command {
            Some(Command::Config { action: Some(ConfigAction::Set { key, value }) }) => {
                assert_eq!(key, "sound.enabled");
                assert_eq!(value, "false");
            }
            _ => panic!("expected Config Set"),
        }
    }
}
