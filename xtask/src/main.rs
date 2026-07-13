use std::path::{Path, PathBuf};

const TEMPLATE: &str = include_str!("../templates/config-command.md");

struct Target {
    harness: &'static str,
    relative_path: &'static str,
    style: Style,
}

enum Style {
    Command,
    Skill,
}

const TARGETS: [Target; 10] = [
    Target { harness: "claude-code", relative_path: "claude-code/.claude/commands/harness-notify-config.md", style: Style::Command },
    Target { harness: "opencode", relative_path: "opencode/.opencode/commands/harness-notify-config.md", style: Style::Command },
    Target { harness: "cursor", relative_path: "cursor/.cursor/commands/harness-notify-config.md", style: Style::Command },
    Target { harness: "antigravity", relative_path: "antigravity/.agents/skills/harness-notify-config/SKILL.md", style: Style::Skill },
    Target { harness: "kimi", relative_path: "kimi/.kimi/skills/harness-notify-config/SKILL.md", style: Style::Skill },
    Target { harness: "kilo", relative_path: "kilo/.kilo/skills/harness-notify-config/SKILL.md", style: Style::Skill },
    Target { harness: "kiro", relative_path: "kiro/.kiro/skills/harness-notify-config/SKILL.md", style: Style::Skill },
    Target { harness: "windsurf", relative_path: "windsurf/.windsurf/skills/harness-notify-config/SKILL.md", style: Style::Skill },
    Target { harness: "cline", relative_path: "cline/.clinerules/harness-notify-skills/harness-notify-config/SKILL.md", style: Style::Skill },
    Target { harness: "copilot", relative_path: "copilot/.github/harness-notify-skills/harness-notify-config/SKILL.md", style: Style::Skill },
];

fn render(style: &Style) -> String {
    match style {
        Style::Command => format!(
            "---\ndescription: Change harness-notify's notification settings\n---\n\n{TEMPLATE}"
        ),
        Style::Skill => format!(
            "---\nname: harness-notify-config\ndescription: Change harness-notify's notification settings\n---\n\n{TEMPLATE}"
        ),
    }
}

fn generate(dist_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for target in &TARGETS {
        let path = dist_root.join(target.relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, render(&target.style))?;
        println!("xtask: {} -> {}", target.harness, path.display());
        written.push(path);
    }
    Ok(written)
}

fn main() {
    let dist_root = PathBuf::from("dist");
    match generate(&dist_root) {
        Ok(written) => println!("xtask: wrote {} config-command artifact(s) under dist/", written.len()),
        Err(e) => {
            eprintln!("xtask: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn generates_a_command_style_artifact_for_claude_code() {
        let dir = tempdir().unwrap();
        generate(dir.path()).unwrap();
        let path = dir.path().join("claude-code/.claude/commands/harness-notify-config.md");
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.starts_with("---\n"));
        assert!(text.contains("description:"));
        assert!(text.contains("harness-notify config set"));
    }

    #[test]
    fn generates_a_skill_style_artifact_for_a_tier_d_harness() {
        let dir = tempdir().unwrap();
        generate(dir.path()).unwrap();
        let path = dir.path().join("cline/.clinerules/harness-notify-skills/harness-notify-config/SKILL.md");
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.starts_with("---\n"));
        assert!(text.contains("name: harness-notify-config"));
    }

    #[test]
    fn generates_all_ten_harnesses() {
        let dir = tempdir().unwrap();
        let written = generate(dir.path()).unwrap();
        assert_eq!(written.len(), 10);
    }
}
