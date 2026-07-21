use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallScope {
    User,
    Project,
}

pub fn resolve_base_dir(
    harness: &str,
    scope: InstallScope,
    home: &Path,
    config_dir: &Path,
    cwd: &Path,
) -> PathBuf {
    // Defense-in-depth: a harness id with path separators (../, /, \) could
    // escape the intended base directory if it reached this fallback. The
    // caller validates the harness against all_adapters() before I/O, but
    // rejecting it here too closes the gap at the lowest-possible layer.
    if harness.contains('/') || harness.contains('\\') || harness.contains("..") {
        return home.join(".harness-notify");
    }
    match (harness, scope) {
        ("claude-code", InstallScope::User) => home.join(".claude"),
        ("claude-code", InstallScope::Project) => cwd.join(".claude"),
        ("opencode", InstallScope::User) => config_dir.join("opencode"),
        ("opencode", InstallScope::Project) => cwd.join(".opencode"),
        ("kimi", InstallScope::User) => home.join(".kimi-code"),
        ("kimi", InstallScope::Project) => cwd.join(".kimi-code"),
        ("antigravity", InstallScope::User) => home.join(".gemini").join("config"),
        ("antigravity", InstallScope::Project) => cwd.join(".agents"),
        (other, InstallScope::User) => home.join(format!(".{other}")),
        (other, InstallScope::Project) => cwd.join(format!(".{other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn claude_code_user_scope_is_home_dot_claude() {
        let got = resolve_base_dir(
            "claude-code", InstallScope::User,
            Path::new("/home/op"), Path::new("/home/op/.config"), Path::new("/proj"),
        );
        assert_eq!(got, Path::new("/home/op/.claude"));
    }

    #[test]
    fn claude_code_project_scope_is_cwd_dot_claude() {
        let got = resolve_base_dir(
            "claude-code", InstallScope::Project,
            Path::new("/home/op"), Path::new("/home/op/.config"), Path::new("/proj"),
        );
        assert_eq!(got, Path::new("/proj/.claude"));
    }

    #[test]
    fn opencode_user_scope_uses_config_dir_not_home() {
        let got = resolve_base_dir(
            "opencode", InstallScope::User,
            Path::new("/home/op"), Path::new("/home/op/.config"), Path::new("/proj"),
        );
        assert_eq!(got, Path::new("/home/op/.config/opencode"));
    }

    #[test]
    fn kimi_user_scope_is_home_dot_kimi_code() {
        let got = resolve_base_dir(
            "kimi", InstallScope::User,
            Path::new("/home/op"), Path::new("/home/op/.config"), Path::new("/proj"),
        );
        assert_eq!(got, Path::new("/home/op/.kimi-code"));
    }
}
