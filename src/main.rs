mod adapters;
mod cli;
mod config;
mod config_ops;
mod events;
mod gui;
mod notify;
mod os_check;
mod resolve;

use adapters::all_adapters;
use clap::Parser;
use cli::{Cli, Command, ConfigAction};
use config::{default_config_path, load_config, save_config};
use events::CanonicalEvent;
use notify::{handle_notify, refine_event_from_notification_type, NotifyContext, RealNotifier};
use resolve::{resolve_base_dir, InstallScope};
use std::io::IsTerminal;
use std::str::FromStr;

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind;
            // A malformed `notify` invocation must never hard-exit: every hook
            // calls it unattended, so a bad/missing flag has to be a silent
            // no-op rather than a clap error that could block the harness's
            // own hook chain. Every other subcommand (and bad global usage)
            // keeps clap's normal informative-error, non-zero-exit behavior.
            // An explicit --help/--version request is not malformed input, so
            // honor it even for `notify` (clap exits 0 for those kinds, which
            // keeps notify's exit-0 guarantee intact).
            let is_help_or_version =
                matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion);
            let is_notify_call = std::env::args().nth(1).as_deref() == Some("notify");
            if is_notify_call && !is_help_or_version {
                std::process::exit(0);
            }
            e.exit();
        }
    };
    match cli.command {
        None => gui::run(),
        Some(Command::Notify { harness, event, title, message, cwd }) => {
            // Never fail hard: an unrecognized --event is a silent no-op,
            // not a crash that could break the calling harness's hook chain.
            if let Ok(mut canonical) = CanonicalEvent::from_str(&event) {
                let payload = read_hook_payload();
                if let Some(notification_type) = payload
                    .as_ref()
                    .and_then(|p| p.get("notification_type"))
                    .and_then(|v| v.as_str())
                {
                    canonical = refine_event_from_notification_type(notification_type, canonical);
                }
                let payload_cwd = payload
                    .as_ref()
                    .and_then(|p| p.get("cwd"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let effective_cwd = cwd.or(payload_cwd);

                let cfg = load_config(&default_config_path());
                let notifier = RealNotifier;
                let now = chrono::Local::now().time();
                let ctx = NotifyContext {
                    title: title.as_deref(),
                    message: message.as_deref(),
                    cwd: effective_cwd.as_deref(),
                };
                handle_notify(&cfg, harness.as_deref().unwrap_or(""), canonical, ctx, &notifier, now);
            }
        }
        Some(Command::Install { harness, project }) => {
            run_install(&harness, project, true);
        }
        Some(Command::Uninstall { harness, project }) => {
            run_install(&harness, project, false);
        }
        Some(Command::Test { harness }) => {
            let cfg = load_config(&default_config_path());
            let notifier = RealNotifier;
            let now = chrono::Local::now().time();
            let cwd = std::env::current_dir().ok().map(|p| p.display().to_string());
            let ctx = NotifyContext { message: Some("Sample notification"), cwd: cwd.as_deref(), ..Default::default() };
            handle_notify(&cfg, harness.as_deref().unwrap_or("test"), CanonicalEvent::Done, ctx, &notifier, now);
        }
        Some(Command::Check { .. }) => run_check(),
        Some(Command::Config { action: None }) => gui::run(),
        Some(Command::Config { action: Some(action) }) => run_config(action),
    }
    // notify/install/uninstall/test/config all print their own errors;
    // the process itself always exits 0 so a stale hook call never blocks
    // the calling harness.
    std::process::exit(0);
}

fn run_install(harness: &str, project: bool, install: bool) {
    let scope = if project { InstallScope::Project } else { InstallScope::User };
    let Some(home) = dirs::home_dir() else {
        eprintln!("harness-notify: could not resolve home directory");
        return;
    };
    let config_dir = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
    let Ok(cwd) = std::env::current_dir() else {
        eprintln!("harness-notify: could not resolve current directory");
        return;
    };
    let base_dir = resolve_base_dir(harness, scope, &home, &config_dir, &cwd);
    let binary_path = std::env::current_exe().unwrap_or_else(|_| "harness-notify".into());

    let adapters = all_adapters();
    let Some(adapter) = adapters.iter().find(|a| a.id() == harness) else {
        eprintln!("unknown harness: {harness}");
        return;
    };
    let result = if install {
        adapter.install(&base_dir, &binary_path)
    } else {
        adapter.uninstall(&base_dir)
    };
    match result {
        Ok(()) => println!("harness-notify: {} {}", if install { "installed for" } else { "uninstalled from" }, harness),
        Err(e) => eprintln!("harness-notify: {e}"),
    }
}

/// Reads and parses the hook's JSON payload from stdin, if one was piped in.
/// Never blocks: a terminal stdin (no piped input, e.g. a manual `notify`
/// call) is skipped entirely rather than waiting for input that will never
/// arrive. Malformed or absent JSON is `None`, not an error - the static
/// --event/--cwd flags already work without it.
fn read_hook_payload() -> Option<serde_json::Value> {
    if std::io::stdin().is_terminal() {
        return None;
    }
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).ok()?;
    serde_json::from_str(&buf).ok()
}

fn run_check() {
    // Never blocks session start: prints at most one line, always exits 0.
    if os_check::os_notifications_enabled() == Some(false) {
        println!(
            "harness-notify: OS-level desktop notifications appear to be disabled \
(Windows: Settings > System > Notifications > \"Notifications\"). \
harness-notify's hooks will run without error but nothing will appear \
on screen until this is turned back on."
        );
    }
}

fn run_config(action: ConfigAction) {
    let path = default_config_path();
    let mut cfg = load_config(&path);
    match action {
        ConfigAction::Get { key } => match config_ops::config_get(&cfg, &key) {
            Ok(v) => println!("{v}"),
            Err(e) => eprintln!("{e}"),
        },
        ConfigAction::Set { key, value } => match config_ops::config_set(&mut cfg, &key, &value) {
            Ok(()) => {
                if let Err(e) = save_config(&cfg, &path) {
                    eprintln!("failed to save config: {e}");
                }
            }
            Err(e) => eprintln!("{e}"),
        },
        ConfigAction::List => {
            for (k, v) in config_ops::config_list(&cfg) {
                println!("{k} = {v}");
            }
        }
    }
}
