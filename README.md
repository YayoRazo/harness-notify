# harness-notify

A small, self-contained Rust CLI that fires a native OS desktop notification
when an AI coding harness (Claude Code, opencode, and others) finishes a task
or needs your input. It wires into each harness's own hook/plugin system so
the notification fires automatically as part of that harness's normal
lifecycle - there is no background daemon, no tray icon, and no autostart
entry. Settings (which events notify, sound, quiet hours, whether the
session name is included) live in one shared `config.toml` and can be
changed either by asking the harness agent to run `harness-notify config
set ...` on your behalf, or through a small on-demand native settings
window.

## Support tiers

Support level varies by harness, depending on how well-documented and
confirmed each one's hook surface is. This table reflects exactly what
`src/adapters/mod.rs`'s `all_adapters()` registers - nothing here is
aspirational.

| Tier | Harness | `notify` hook install | Status |
|---|---|---|---|
| A | Claude Code | `harness-notify install --harness claude-code` | Full, tested |
| A | opencode | `harness-notify install --harness opencode` | Full, tested |
| B | Antigravity | `harness-notify install --harness antigravity` | Shipped, **UNVERIFIED** |
| B | Kimi Code CLI | `harness-notify install --harness kimi` | Shipped, **UNVERIFIED** |
| C | Kilo | not supported | Config-command only (see below) |
| C | Kiro | not supported | Config-command only (see below) |
| D | Cursor | not supported | Config-command only (see below) |
| D | Windsurf | not supported | Config-command only (see below) |
| D | Cline | not supported | Config-command only (see below) |
| D | GitHub Copilot | not supported | Config-command only (see below) |

Notes on what these tiers actually mean in the shipped code:

- **Tier A** (`src/adapters/claude_code.rs`, `src/adapters/opencode.rs`):
  `install`/`uninstall` idempotently patch that harness's real hook or
  plugin file. Claude Code gets `Stop` -> `done`, `Notification` ->
  `attention`, `SubagentStop` -> `subagent-done` entries in `settings.json`.
  opencode gets a generated `plugin/harness-notify.js` file (under the
  project's `.opencode/` or the user's `<config_dir>/opencode/`) that
  listens on the generic `event` hook key (`session.idle`/`session.status`
  -> `done`, `permission.asked` -> `attention`).
- **Tier B** (`src/adapters/antigravity.rs`, `src/adapters/kimi.rs`): the
  same idempotent-patch behavior as Tier A, but marked **UNVERIFIED**
  because the exact target path or event coverage has not been confirmed
  against a live install. Antigravity's adapter patches `hooks.json` and
  only wires the confirmed `Stop` -> `done` event - no `attention`-equivalent
  event name could be confirmed, so it is deliberately not invented. Kimi's
  adapter patches `config.toml`'s `[[hooks]]` array with the same
  `Stop`/`Notification`/`SubagentStop` names Claude Code uses, targeting
  `~/.kimi-code/config.toml`; a third-party source elsewhere uses
  `~/.kimi/config.toml` instead, so confirm the path on a real install
  before relying on it.
- **Tiers C and D** (`src/adapters/unsupported.rs`): both are implemented by
  the exact same `UnsupportedAdapter` - there is no behavioral difference
  between them in the code today. `install` and `uninstall` always return a
  clear error (`"<harness> does not support automatic notify-hook install
  yet (open research question, see README)"`) and never write anything. The
  C/D split reflects how each harness
  was researched (Tier D's custom-command/skill surface is confirmed and
  documented; Tier C's is not), not a difference in what the tool does for
  them. All six get a generated config-command artifact instead (see
  "Configuring from inside a harness" below).

The spec this tool was built from also defines a conceptual "Tier E" (no
code, README-only). No shipped harness in `all_adapters()` falls into that
bucket, so it has no row in the table above - only the ten harness ids the
code actually registers are listed, in the order it registers them:
`claude-code`, `opencode`, `antigravity`, `kimi`, `kilo`, `kiro`, `cursor`,
`windsurf`, `cline`, `copilot`.

## Install

Build and install from source with Cargo:

```sh
cargo install --path .
```

This builds the `gui` feature by default (see "GUI feature" below). To skip
the native settings window and its `eframe`/`egui` dependencies:

```sh
cargo install --path . --no-default-features
```

License: dual MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`).

## Command reference

| Command | Flags | What it does |
|---|---|---|
| `harness-notify notify` | `--event <name>` (required), `--harness <id>`, `--title <text>`, `--message <text>` | Fires a notification if the event is enabled and outside quiet hours. This is what an installed hook actually calls. An unrecognized `--event` (or any other malformed input) is a silent no-op, never a crash or a non-zero exit - it must never block the calling harness's hook chain. |
| `harness-notify install` | `--harness <id>` (required), `--project` | Installs the notify hook into the named harness's config. Defaults to the user-level location; `--project` targets the current project directory instead. Fails with a clear message on Tier C/D harnesses. |
| `harness-notify uninstall` | `--harness <id>` (required), `--project` | Removes only the hook entries this tool installed, leaving any other entries in the same file untouched. Fails with a clear message on Tier C/D harnesses. |
| `harness-notify test` | `--harness <id>` | Fires a sample "done" notification immediately, ignoring the harness's own hook wiring - useful for confirming the OS notification path itself works. It still goes through the same `should_fire()` config check as a real notify, so `events.done=false` or active quiet hours will silently suppress it too. |
| `harness-notify config get <key>` | - | Prints one setting's current value. |
| `harness-notify config set <key> <value>` | - | Changes one setting and saves `config.toml`. |
| `harness-notify config list` | - | Prints every current setting. |
| `harness-notify config` (no subcommand) | - | Opens the on-demand native settings window. |
| `harness-notify` (no arguments) | - | Also opens the on-demand native settings window. |

Only `notify` is guaranteed to always exit `0`: because hooks call it
unattended, a malformed or missing flag is a silent no-op rather than a
clap error, so it can never block the calling harness's hook chain. The
other subcommands (`install`, `uninstall`, `test`, `config`) use clap's
normal error handling on malformed input - an informative message to
stderr and a non-zero exit code - since a human or an agent runs them
interactively, not an unattended hook.

### Config keys

| Key | Values | Meaning |
|---|---|---|
| `events.done` | `true`/`false` | notify when a task/session finishes (default `true`) |
| `events.attention` | `true`/`false` | notify when input/a decision is needed (default `true`) |
| `events.subagent_done` | `true`/`false` | notify when a subagent finishes (default `false`) |
| `session.include_name` | `true`/`false` | include the harness name in the message (default `false`) |
| `session.format` | `name` or `path` | how the session is labeled, if included (default `name`) |
| `sound.enabled` | `true`/`false` | play a sound with the notification (default `true`) |
| `dnd.enabled` | `true`/`false` | enable quiet hours (default `false`) |
| `dnd.start` | `HH:MM` | quiet hours start, local time (default `22:00`) |
| `dnd.end` | `HH:MM` | quiet hours end, local time, may cross midnight (default `08:00`) |

Config lives at `~/.harness-notify/config.toml` (a single shared file - v1
has no per-harness overrides). A missing or corrupt file silently falls
back to the defaults above rather than erroring.

## Platform notes

Notifications are delivered through [`notify-rust`](https://docs.rs/notify-rust):

- **Windows**: uses the WinRT toast notification API.
- **macOS**: uses Notification Center.
- **Linux**: uses the freedesktop D-Bus notification spec, which requires a
  running notification daemon (provided by most desktop environments; on a
  minimal or headless setup you may need to run one yourself, e.g.
  `dunst`).

## GUI feature

The on-demand native settings window (`src/gui.rs`, built on `eframe`/`egui`)
is a default-on Cargo feature named `gui`. It opens whenever `harness-notify`
is run with no arguments, or as `harness-notify config` with no further
subcommand, and lets you toggle every setting from the config table above
with a "Save" button - no command line needed. It is not a background
process: the window opens on demand and exits when closed.

Build without it with `cargo build --no-default-features` (or `cargo install
--path . --no-default-features`). A build made this way still works fully
from the command line; running it with no arguments prints a message telling
you to use `harness-notify config get/set/list` instead of opening a window.

## Configuring from inside a harness

Because settings live in one `config.toml` reachable only through the
`harness-notify config get/set/list` commands, the natural way to change a
setting from inside a harness session is to ask the coding agent in plain
language (e.g. "mute subagent notifications", "set quiet hours from 10pm to
8am") and let it translate that into the right `config` call. To make that
reliable, the `xtask` workspace member generates a small, harness-appropriate
command/skill artifact describing exactly that mapping, for **all ten**
harnesses - including the six that don't get a notify-hook install.

Generate the artifacts with:

```sh
cargo run -p xtask
```

This writes one file per harness under `dist/` (gitignored, regenerate as
needed - the canonical content lives in `xtask/templates/config-command.md`).
Copy the generated file into the matching harness-specific location in your
project or home directory:

| Harness | Generated artifact | Copy to |
|---|---|---|
| Claude Code | `dist/claude-code/.claude/commands/harness-notify-config.md` | `.claude/commands/` (project) or `~/.claude/commands/` (user) |
| opencode | `dist/opencode/.opencode/commands/harness-notify-config.md` | `.opencode/commands/` (project) or `<config_dir>/opencode/commands/` (user) |
| Cursor | `dist/cursor/.cursor/commands/harness-notify-config.md` | `.cursor/commands/` |
| Antigravity | `dist/antigravity/.agents/skills/harness-notify-config/SKILL.md` | `.agents/skills/harness-notify-config/` |
| Kimi Code CLI | `dist/kimi/.kimi/skills/harness-notify-config/SKILL.md` | `.kimi/skills/harness-notify-config/` |
| Kilo | `dist/kilo/.kilo/skills/harness-notify-config/SKILL.md` | `.kilo/skills/harness-notify-config/` |
| Kiro | `dist/kiro/.kiro/skills/harness-notify-config/SKILL.md` | `.kiro/skills/harness-notify-config/` |
| Windsurf | `dist/windsurf/.windsurf/skills/harness-notify-config/SKILL.md` | `.windsurf/skills/harness-notify-config/` |
| Cline | `dist/cline/.clinerules/harness-notify-skills/harness-notify-config/SKILL.md` | `.clinerules/harness-notify-skills/harness-notify-config/` |
| GitHub Copilot | `dist/copilot/.github/harness-notify-skills/harness-notify-config/SKILL.md` | `.github/harness-notify-skills/harness-notify-config/` |

Claude Code, opencode, and Cursor get a command-style artifact (front matter
with a `description` field); every other harness gets a skill-style artifact
(front matter with `name` and `description`). Both styles carry the same
body: the three `harness-notify config` subcommands, the full key/value
table above, and one worked example
(`"mute subagent notifications" -> harness-notify config set
events.subagent_done false`).

This works the same way regardless of tier - a Tier C/D harness with no
notify-hook support still gets full, agent-mediated configurability through
its command/skill artifact; only the automatic hook `install` is unavailable
for those six.
