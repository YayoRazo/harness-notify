// UNVERIFIED (Tier B): event names (Stop/Notification/SubagentStop) and the
// [[hooks]] config.toml shape are confirmed against Kimi Code CLI's own docs
// (kimi-cli.com/en/customization/hooks.html). The config file's exact
// directory is not: this adapter targets ~/.kimi-code/config.toml, but a
// third-party example elsewhere uses ~/.kimi/config.toml. Confirm against a
// live install before removing this notice. TOML round-trip through the
// `toml` crate does not preserve
// comments/formatting in the rest of the user's config.toml - acceptable for
// v1, called out in the README.
use super::{backup_before_write, HookAdapter};
use crate::events::CanonicalEvent;
use std::path::{Path, PathBuf};

pub struct KimiAdapter;

const HOOK_MAP: [(&str, CanonicalEvent); 3] = [
    ("Stop", CanonicalEvent::Done),
    ("Notification", CanonicalEvent::Attention),
    ("SubagentStop", CanonicalEvent::SubagentDone),
];

// Fixed CLI-args marker, not tied to binary_path's text: a check based on the
// binary_path text would silently break if the binary is ever renamed or
// aliased. Every command this adapter emits contains this exact substring
// regardless of what harness-notify's own executable is named.
const MARKER: &str = "notify --harness kimi --event";

fn config_path(base_dir: &Path) -> PathBuf {
    base_dir.join("config.toml")
}

fn our_command(binary_path: &Path, event: &str) -> String {
    format!("\"{}\" notify --harness kimi --event {}", binary_path.display(), event)
}

fn read_root(path: &Path) -> Result<toml::Value, String> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(toml::Value::Table(toml::map::Map::new()));
    }
    toml::from_str(&text).map_err(|e| e.to_string())
}

fn is_ours(entry: &toml::Value) -> bool {
    entry.get("command").and_then(|c| c.as_str()).map(|c| c.contains(MARKER)).unwrap_or(false)
}

fn write_root(path: &Path, root: &toml::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, toml::to_string_pretty(root).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn patch(base_dir: &Path, binary_path: Option<&Path>) -> Result<(), String> {
    let path = config_path(base_dir);
    backup_before_write(&path).map_err(|e| e.to_string())?;
    let mut root = read_root(&path)?;
    let table = root.as_table_mut().ok_or("config.toml root must be a table")?;
    let mut hooks: Vec<toml::Value> = table
        .get("hooks")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    hooks.retain(|h| !is_ours(h));
    if let Some(bin) = binary_path {
        for (event_name, canonical) in HOOK_MAP {
            let mut entry = toml::map::Map::new();
            entry.insert("event".to_string(), toml::Value::String(event_name.to_string()));
            entry.insert("command".to_string(), toml::Value::String(our_command(bin, canonical.as_str())));
            hooks.push(toml::Value::Table(entry));
        }
    }
    table.insert("hooks".to_string(), toml::Value::Array(hooks));
    write_root(&path, &root)
}

impl HookAdapter for KimiAdapter {
    fn id(&self) -> &'static str {
        "kimi"
    }

    fn install(&self, base_dir: &Path, binary_path: &Path) -> Result<(), String> {
        patch(base_dir, Some(binary_path))
    }

    fn uninstall(&self, base_dir: &Path) -> Result<(), String> {
        patch(base_dir, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn config_path(base: &std::path::Path) -> std::path::PathBuf {
        base.join("config.toml")
    }

    #[test]
    fn install_writes_three_hooks_entries() {
        let dir = tempdir().unwrap();
        let adapter = KimiAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        let text = std::fs::read_to_string(config_path(dir.path())).unwrap();
        let root: toml::Value = toml::from_str(&text).unwrap();
        let hooks = root["hooks"].as_array().unwrap();
        let ours: Vec<_> = hooks.iter().filter(|h| {
            h["command"].as_str().unwrap_or("").contains("harness-notify")
        }).collect();
        assert_eq!(ours.len(), 3);
        assert!(text.contains("event = \"Stop\""));
        assert!(text.contains("event = \"Notification\""));
        assert!(text.contains("event = \"SubagentStop\""));
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempdir().unwrap();
        let adapter = KimiAdapter;
        let bin = std::path::Path::new("/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.install(dir.path(), bin).unwrap();
        let text = std::fs::read_to_string(config_path(dir.path())).unwrap();
        let root: toml::Value = toml::from_str(&text).unwrap();
        let ours = root["hooks"].as_array().unwrap().iter()
            .filter(|h| h["command"].as_str().unwrap_or("").contains("harness-notify"))
            .count();
        assert_eq!(ours, 3);
    }

    #[test]
    fn install_preserves_a_foreign_hooks_entry() {
        let dir = tempdir().unwrap();
        std::fs::write(
            config_path(dir.path()),
            "[[hooks]]\nevent = \"PreToolUse\"\ncommand = \"some-other-tool\"\n",
        ).unwrap();
        let adapter = KimiAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        let text = std::fs::read_to_string(config_path(dir.path())).unwrap();
        assert!(text.contains("some-other-tool"));
    }

    #[test]
    fn uninstall_removes_only_our_entries() {
        let dir = tempdir().unwrap();
        let adapter = KimiAdapter;
        let bin = std::path::Path::new("/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.uninstall(dir.path()).unwrap();
        let text = std::fs::read_to_string(config_path(dir.path())).unwrap();
        assert!(!text.contains("harness-notify"));
    }
}
