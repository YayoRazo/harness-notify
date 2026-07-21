use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EventsConfig {
    pub done: bool,
    pub attention: bool,
    pub subagent_done: bool,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self { done: true, attention: true, subagent_done: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SessionConfig {
    pub include_name: bool,
    pub format: String,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { include_name: false, format: "name".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SoundConfig {
    pub enabled: bool,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DndConfig {
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

impl Default for DndConfig {
    fn default() -> Self {
        Self { enabled: false, start: "22:00".to_string(), end: "08:00".to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Config {
    pub events: EventsConfig,
    pub session: SessionConfig,
    pub sound: SoundConfig,
    pub dnd: DndConfig,
}

/// `HARNESS_NOTIFY_CONFIG_DIR` overrides the directory (tests point it at a
/// temp dir so they never touch the real user config; users can relocate it
/// the same way).
pub fn default_config_path() -> PathBuf {
    std::env::var_os("HARNESS_NOTIFY_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".harness-notify")
        })
        .join("config.toml")
}

pub fn load_config(path: &Path) -> Config {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn load_config_with_warning(path: &Path) -> (Config, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Config::default(), None),
        Err(e) => return (Config::default(), Some(format!("could not read config: {e}"))),
    };
    if text.trim().is_empty() {
        return (Config::default(), None);
    }
    match toml::from_str(&text) {
        Ok(cfg) => (cfg, None),
        Err(e) => (Config::default(), Some(format!("config is malformed, using defaults: {e}"))),
    }
}

pub fn save_config(cfg: &Config, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(cfg).expect("Config always serializes to TOML");
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = load_config(&path);
        assert!(cfg.events.done);
        assert!(cfg.events.attention);
        assert!(!cfg.events.subagent_done);
        assert!(cfg.sound.enabled);
        assert!(!cfg.dnd.enabled);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "not valid toml {{{").unwrap();
        let cfg = load_config(&path);
        assert!(cfg.events.done);
    }

    #[test]
    fn partial_file_keeps_present_values_and_defaults_the_rest() {
        // A config.toml with only some fields set must load the present
        // values verbatim and fall back to defaults for everything omitted,
        // including whole sections that are absent (session/sound/dnd here).
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "[events]\nsubagent_done = true\n").unwrap();
        let cfg = load_config(&path);
        assert!(cfg.events.subagent_done, "explicitly-set field is kept");
        assert!(cfg.events.done, "omitted field in a present section defaults");
        assert_eq!(cfg.session.format, "name", "absent section defaults");
        assert!(cfg.sound.enabled, "absent section defaults");
        assert_eq!(cfg.dnd.start, "22:00", "absent section defaults");
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut cfg = Config::default();
        cfg.events.subagent_done = true;
        cfg.dnd.enabled = true;
        cfg.dnd.start = "23:00".to_string();
        save_config(&cfg, &path).unwrap();
        let loaded = load_config(&path);
        assert!(loaded.events.subagent_done);
        assert!(loaded.dnd.enabled);
        assert_eq!(loaded.dnd.start, "23:00");
    }
}
