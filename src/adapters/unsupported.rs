use super::HookAdapter;
use std::path::Path;

pub struct UnsupportedAdapter {
    pub harness_id: &'static str,
}

impl HookAdapter for UnsupportedAdapter {
    fn id(&self) -> &'static str {
        self.harness_id
    }

    fn install(&self, _base_dir: &Path, _binary_path: &Path) -> Result<(), String> {
        Err(format!(
            "{} does not support automatic notify-hook install yet (open research question, see README)",
            self.harness_id
        ))
    }

    fn uninstall(&self, _base_dir: &Path) -> Result<(), String> {
        Err(format!("{} has no notify-hook installed by this tool", self.harness_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn install_fails_with_a_clear_message() {
        let dir = tempdir().unwrap();
        let adapter = UnsupportedAdapter { harness_id: "kilo" };
        let err = adapter.install(dir.path(), std::path::Path::new("/bin/harness-notify")).unwrap_err();
        assert!(err.contains("kilo"));
        assert!(err.contains("not"));
    }

    #[test]
    fn uninstall_fails_with_a_clear_message_too() {
        let dir = tempdir().unwrap();
        let adapter = UnsupportedAdapter { harness_id: "cursor" };
        let err = adapter.uninstall(dir.path()).unwrap_err();
        assert!(err.contains("cursor"));
    }

    #[test]
    fn id_reflects_the_configured_harness() {
        let adapter = UnsupportedAdapter { harness_id: "kiro" };
        assert_eq!(adapter.id(), "kiro");
    }
}
