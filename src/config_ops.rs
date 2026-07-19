use crate::config::Config;

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("expected true/false, got {other}")),
    }
}

fn parse_hh_mm(value: &str) -> Result<(), String> {
    chrono::NaiveTime::parse_from_str(value, "%H:%M")
        .map(|_| ())
        .map_err(|_| format!("expected HH:MM, got {value}"))
}

fn parse_session_format(value: &str) -> Result<(), String> {
    match value {
        "name" | "path" => Ok(()),
        other => Err(format!("expected name/path, got {other}")),
    }
}

/// The GUI's pre-save check for the two free-text time fields; the other
/// settings it edits are checkboxes and cannot hold an invalid value.
pub fn validate_dnd_times(cfg: &Config) -> Result<(), String> {
    parse_hh_mm(&cfg.dnd.start)?;
    parse_hh_mm(&cfg.dnd.end)
}

pub fn config_get(cfg: &Config, key: &str) -> Result<String, String> {
    Ok(match key {
        "events.done" => cfg.events.done.to_string(),
        "events.attention" => cfg.events.attention.to_string(),
        "events.subagent_done" => cfg.events.subagent_done.to_string(),
        "session.include_name" => cfg.session.include_name.to_string(),
        "session.format" => cfg.session.format.clone(),
        "sound.enabled" => cfg.sound.enabled.to_string(),
        "dnd.enabled" => cfg.dnd.enabled.to_string(),
        "dnd.start" => cfg.dnd.start.clone(),
        "dnd.end" => cfg.dnd.end.clone(),
        other => return Err(format!("unknown key: {other}")),
    })
}

pub fn config_set(cfg: &mut Config, key: &str, value: &str) -> Result<(), String> {
    match key {
        "events.done" => cfg.events.done = parse_bool(value)?,
        "events.attention" => cfg.events.attention = parse_bool(value)?,
        "events.subagent_done" => cfg.events.subagent_done = parse_bool(value)?,
        "session.include_name" => cfg.session.include_name = parse_bool(value)?,
        "session.format" => {
            parse_session_format(value)?;
            cfg.session.format = value.to_string();
        }
        "sound.enabled" => cfg.sound.enabled = parse_bool(value)?,
        "dnd.enabled" => cfg.dnd.enabled = parse_bool(value)?,
        "dnd.start" => {
            parse_hh_mm(value)?;
            cfg.dnd.start = value.to_string();
        }
        "dnd.end" => {
            parse_hh_mm(value)?;
            cfg.dnd.end = value.to_string();
        }
        other => return Err(format!("unknown key: {other}")),
    }
    Ok(())
}

pub fn config_list(cfg: &Config) -> Vec<(String, String)> {
    let keys = [
        "events.done", "events.attention", "events.subagent_done",
        "session.include_name", "session.format",
        "sound.enabled", "dnd.enabled", "dnd.start", "dnd.end",
    ];
    keys.iter()
        .map(|k| (k.to_string(), config_get(cfg, k).expect("key list is exhaustive")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn get_and_set_a_bool_key() {
        let mut cfg = Config::default();
        assert_eq!(config_get(&cfg, "events.done").unwrap(), "true");
        config_set(&mut cfg, "events.done", "false").unwrap();
        assert_eq!(config_get(&cfg, "events.done").unwrap(), "false");
    }

    #[test]
    fn get_and_set_a_string_key() {
        let mut cfg = Config::default();
        config_set(&mut cfg, "dnd.start", "23:30").unwrap();
        assert_eq!(config_get(&cfg, "dnd.start").unwrap(), "23:30");
    }

    #[test]
    fn unknown_key_is_an_error() {
        let cfg = Config::default();
        assert!(config_get(&cfg, "nope.nope").is_err());
        let mut cfg2 = Config::default();
        assert!(config_set(&mut cfg2, "nope.nope", "x").is_err());
    }

    #[test]
    fn invalid_bool_value_is_an_error() {
        let mut cfg = Config::default();
        assert!(config_set(&mut cfg, "events.done", "not-a-bool").is_err());
    }

    #[test]
    fn invalid_session_format_is_an_error() {
        let mut cfg = Config::default();
        assert!(config_set(&mut cfg, "session.format", "pth").is_err());
        assert!(config_set(&mut cfg, "session.format", "path").is_ok());
    }

    #[test]
    fn invalid_dnd_time_is_an_error_and_leaves_the_value_unchanged() {
        let mut cfg = Config::default();
        assert!(config_set(&mut cfg, "dnd.start", "10pm").is_err());
        assert_eq!(cfg.dnd.start, "22:00");
        assert!(config_set(&mut cfg, "dnd.end", "24:00").is_err());
        assert!(config_set(&mut cfg, "dnd.start", "23:30").is_ok());
    }

    #[test]
    fn validate_dnd_times_flags_a_bad_time() {
        let mut cfg = Config::default();
        assert!(validate_dnd_times(&cfg).is_ok());
        cfg.dnd.end = "8 am".to_string();
        assert!(validate_dnd_times(&cfg).is_err());
    }

    #[test]
    fn list_returns_every_key() {
        let cfg = Config::default();
        let list = config_list(&cfg);
        let keys: Vec<&str> = list.iter().map(|(k, _)| k.as_str()).collect();
        for expected in [
            "events.done", "events.attention", "events.subagent_done",
            "session.include_name", "session.format",
            "sound.enabled", "dnd.enabled", "dnd.start", "dnd.end",
        ] {
            assert!(keys.contains(&expected), "missing key {expected}");
        }
    }

    #[test]
    fn set_accepts_every_key_that_list_emits() {
        // The 9-key set is enumerated separately in get/set/list. This guards
        // the one divergence the other tests don't: a key present in list/get
        // but missing from set. Feeding each listed value straight back into
        // config_set must round-trip without an "unknown key" error.
        let cfg = Config::default();
        for (key, value) in config_list(&cfg) {
            let mut c = Config::default();
            assert!(
                config_set(&mut c, &key, &value).is_ok(),
                "config_set rejected a key config_list emits: {key}"
            );
        }
    }
}
