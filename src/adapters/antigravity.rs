// UNVERIFIED (Tier B): "Stop fires upon agent termination" per Google's own
// Antigravity docs, which target this hooks.json. No confirmed "needs
// input"/Notification event name was found in any available source, so
// `attention` is deliberately NOT wired here - do not guess an event name.
// Revisit once a live install can be inspected.
use super::{backup_before_write, HookAdapter};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub struct AntigravityAdapter;

// Fixed CLI-args marker, not tied to binary_path's text: a check based on the
// binary_path text would silently break if the binary is ever renamed or
// aliased.
const MARKER: &str = "notify --harness antigravity --event";

fn hooks_path(base_dir: &Path) -> PathBuf {
    base_dir.join("hooks.json")
}

fn our_command(binary_path: &Path) -> String {
    format!("\"{}\" notify --harness antigravity --event done", binary_path.display())
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

fn patch(base_dir: &Path, binary_path: Option<&Path>) -> Result<(), String> {
    let path = hooks_path(base_dir);
    // Uninstall with no hooks.json present: nothing to remove, and writing
    // would materialize a file (with an empty Stop array) that never existed.
    if binary_path.is_none() && !path.exists() {
        return Ok(());
    }
    backup_before_write(&path).map_err(|e| e.to_string())?;
    let mut root = read_root(&path)?;
    let obj = root.as_object_mut().ok_or("hooks.json root must be an object")?;
    let arr = obj
        .entry("Stop")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or("\"Stop\" is not a JSON array")?;
    arr.retain(|h| !h["command"].as_str().unwrap_or("").contains(MARKER));
    if let Some(bin) = binary_path {
        arr.push(json!({ "command": our_command(bin) }));
    }
    write_root(&path, &root)
}

impl HookAdapter for AntigravityAdapter {
    fn id(&self) -> &'static str {
        "antigravity"
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

    fn hooks_path(base: &std::path::Path) -> std::path::PathBuf {
        base.join("hooks.json")
    }

    #[test]
    fn install_wires_only_the_confirmed_stop_event() {
        let dir = tempdir().unwrap();
        let adapter = AntigravityAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        let text = std::fs::read_to_string(hooks_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(root["Stop"].as_array().unwrap().iter().any(|h| {
            h["command"].as_str().unwrap_or("").contains("harness-notify")
        }));
        assert!(root.get("Notification").is_none(), "no confirmed attention-equivalent event exists yet");
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempdir().unwrap();
        let adapter = AntigravityAdapter;
        let bin = std::path::Path::new("/bin/harness-notify");
        adapter.install(dir.path(), bin).unwrap();
        adapter.install(dir.path(), bin).unwrap();
        let text = std::fs::read_to_string(hooks_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["Stop"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn uninstall_without_a_hooks_file_creates_nothing() {
        let dir = tempdir().unwrap();
        let adapter = AntigravityAdapter;
        adapter.uninstall(dir.path()).unwrap();
        assert!(!hooks_path(dir.path()).exists());
    }

    #[test]
    fn uninstall_removes_only_our_entry() {
        let dir = tempdir().unwrap();
        let adapter = AntigravityAdapter;
        adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap();
        adapter.uninstall(dir.path()).unwrap();
        let text = std::fs::read_to_string(hooks_path(dir.path())).unwrap();
        let root: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(root["Stop"].as_array().unwrap().len(), 0);
    }
}
