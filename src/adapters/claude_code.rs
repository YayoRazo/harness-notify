use super::{backup_before_write, HookAdapter};
use crate::events::CanonicalEvent;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct ClaudeCodeAdapter;

const HOOK_MAP: [(&str, CanonicalEvent); 3] = [
    ("Stop", CanonicalEvent::Done),
    ("Notification", CanonicalEvent::Attention),
    ("SubagentStop", CanonicalEvent::SubagentDone),
];

fn settings_path(base_dir: &Path) -> PathBuf {
    base_dir.join("settings.json")
}

/// Fixed argument syntax `our_command` always emits, independent of `binary_path`'s
/// own text. Used to detect our own hook entries regardless of how the binary is
/// named, aliased, or invoked.
const MARKER: &str = "notify --harness claude-code --event";

/// Same idea for the SessionStart entry - a distinctive, multi-word marker
/// from our own fixed argument list, not a single generic word like "check"
/// that a foreign hook could plausibly also contain.
const CHECK_MARKER: &str = "check --hook session-start";

fn our_command(binary_path: &Path, event: &str) -> String {
    format!("\"{}\" notify --harness claude-code --event {}", binary_path.display(), event)
}

fn our_check_command(binary_path: &Path) -> String {
    format!("\"{}\" check --hook session-start", binary_path.display())
}

fn patch(root: &mut Value, hook_name: &str, marker: &str, command: Option<String>) -> Result<(), String> {
    let hooks = root
        .as_object_mut()
        .ok_or_else(|| "settings.json root is not a JSON object".to_string())?
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let hooks_obj = hooks
        .as_object_mut()
        .ok_or_else(|| "\"hooks\" is not a JSON object".to_string())?;
    let arr = hooks_obj
        .entry(hook_name)
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| format!("\"{hook_name}\" is not a JSON array"))?;
    arr.retain(|group| {
        !group["hooks"]
            .as_array()
            .map(|h| h.iter().any(|entry| {
                entry["command"].as_str().map(|c| c.contains(marker)).unwrap_or(false)
            }))
            .unwrap_or(false)
    });
    if let Some(cmd) = command {
        arr.push(json!({ "hooks": [ { "type": "command", "command": cmd } ] }));
    }
    Ok(())
}

fn read_root(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn write_root(path: &Path, root: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, serde_json::to_string_pretty(root).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

impl HookAdapter for ClaudeCodeAdapter {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn install(&self, base_dir: &Path, binary_path: &Path) -> Result<(), String> {
        let path = settings_path(base_dir);
        backup_before_write(&path).map_err(|e| e.to_string())?;
        let mut root = read_root(&path)?;
        for (hook_name, event) in HOOK_MAP {
            patch(&mut root, hook_name, MARKER, Some(our_command(binary_path, event.as_str())))?;
        }
        patch(&mut root, "SessionStart", CHECK_MARKER, Some(our_check_command(binary_path)))?;
        write_root(&path, &root)
    }

    fn uninstall(&self, base_dir: &Path) -> Result<(), String> {
        let path = settings_path(base_dir);
        // Nothing to remove - and writing here would materialize a
        // settings.json (with empty hook arrays) that never existed.
        if !path.exists() {
            return Ok(());
        }
        backup_before_write(&path).map_err(|e| e.to_string())?;
        let mut root = read_root(&path)?;
        for (hook_name, _) in HOOK_MAP {
            patch(&mut root, hook_name, MARKER, None)?;
        }
        patch(&mut root, "SessionStart", CHECK_MARKER, None)?;
        write_root(&path, &root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn settings_path(base: &std::path::Path) -> std::path::PathBuf {
        base.join("settings.json")
    }

    #[test]
    fn install_writes_all_three_hooks() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        adapter.install(dir.path(), std::path::Path::new("/usr/local/bin/harness-notify")).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        for event in ["Stop", "Notification", "SubagentStop"] {
            let arr = root["hooks"][event].as_array().unwrap();
            assert_eq!(arr.len(), 1, "{event} should have exactly one group");
        }
        assert!(text.contains("--event done"));
        assert!(text.contains("--event attention"));
        assert!(text.contains("--event subagent-done"));
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        let bin = std::path::Path::new("/usr/local/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.install(dir.path(), bin).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["hooks"]["Stop"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn install_preserves_foreign_hooks() {
        let dir = tempdir().unwrap();
        std::fs::write(
            settings_path(dir.path()),
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"some-other-tool notify --harness other --event whatever"}]}]}}"#,
        ).unwrap();
        let adapter = ClaudeCodeAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        assert!(text.contains("some-other-tool notify --harness other --event whatever"));
        assert!(text.contains("harness-notify"));
    }

    #[test]
    fn uninstall_removes_only_our_entries() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        let bin = std::path::Path::new("/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.uninstall(dir.path()).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["hooks"]["Stop"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn uninstall_without_a_settings_file_creates_nothing() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        adapter.uninstall(dir.path()).unwrap();
        assert!(!settings_path(dir.path()).exists());
    }

    #[test]
    fn install_backs_up_existing_file() {
        let dir = tempdir().unwrap();
        std::fs::write(settings_path(dir.path()), "{}").unwrap();
        let adapter = ClaudeCodeAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        assert!(dir.path().join("settings.json.bak").exists());
    }

    #[test]
    fn install_errs_instead_of_panicking_on_malformed_hooks_shape() {
        let dir = tempdir().unwrap();
        std::fs::write(settings_path(dir.path()), r#"{"hooks":"not-an-object"}"#).unwrap();
        let adapter = ClaudeCodeAdapter;
        let result = adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify"));
        assert!(result.is_err());
    }

    #[test]
    fn install_wires_session_start_check() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
        assert!(text.contains("check --hook session-start"));
    }

    #[test]
    fn session_start_check_is_idempotent() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        let bin = std::path::Path::new("/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.install(dir.path(), bin).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn uninstall_removes_session_start_check_too() {
        let dir = tempdir().unwrap();
        let adapter = ClaudeCodeAdapter;
        let bin = std::path::Path::new("/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.uninstall(dir.path()).unwrap();
        let text = std::fs::read_to_string(settings_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 0);
    }
}
