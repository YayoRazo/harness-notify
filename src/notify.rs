use crate::config::Config;
use crate::events::CanonicalEvent;
use chrono::NaiveTime;
#[cfg(test)]
use std::cell::RefCell;

pub trait Notifier {
    fn fire(&self, title: &str, message: &str) -> Result<(), String>;
}

pub struct RealNotifier;

impl Notifier for RealNotifier {
    fn fire(&self, title: &str, message: &str) -> Result<(), String> {
        notify_rust::Notification::new()
            .summary(title)
            .body(message)
            .show()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

/// Test-only in-memory notifier that records every fire() call instead of
/// showing an OS notification. Gated to test builds so it is not dead code in
/// a released binary.
#[cfg(test)]
#[derive(Default)]
pub struct FakeNotifier {
    pub calls: RefCell<Vec<(String, String)>>,
}

#[cfg(test)]
impl Notifier for FakeNotifier {
    fn fire(&self, title: &str, message: &str) -> Result<(), String> {
        self.calls.borrow_mut().push((title.to_string(), message.to_string()));
        Ok(())
    }
}

fn parse_time(s: &str) -> NaiveTime {
    NaiveTime::parse_from_str(s, "%H:%M").unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap())
}

fn in_quiet_hours(cfg: &Config, now: NaiveTime) -> bool {
    let start = parse_time(&cfg.dnd.start);
    let end = parse_time(&cfg.dnd.end);
    if start <= end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

pub fn should_fire(cfg: &Config, event: CanonicalEvent, now: NaiveTime) -> bool {
    let event_enabled = match event {
        CanonicalEvent::Done => cfg.events.done,
        CanonicalEvent::Attention => cfg.events.attention,
        CanonicalEvent::SubagentDone => cfg.events.subagent_done,
    };
    if !event_enabled {
        return false;
    }
    if cfg.dnd.enabled && in_quiet_hours(cfg, now) {
        return false;
    }
    true
}

fn default_title(event: CanonicalEvent) -> &'static str {
    match event {
        CanonicalEvent::Done => "Task complete",
        CanonicalEvent::Attention => "Needs your input",
        CanonicalEvent::SubagentDone => "Subagent finished",
    }
}

pub fn handle_notify(
    cfg: &Config,
    harness: &str,
    event: CanonicalEvent,
    title_override: Option<&str>,
    message_override: Option<&str>,
    notifier: &dyn Notifier,
    now: NaiveTime,
) {
    if !should_fire(cfg, event, now) {
        return;
    }
    let title = title_override.unwrap_or_else(|| default_title(event));
    let harness_label = if harness.is_empty() { "unknown" } else { harness };
    let message = match (message_override, cfg.session.include_name) {
        (Some(m), true) => format!("{m} ({harness_label})"),
        (Some(m), false) => m.to_string(),
        (None, true) => harness_label.to_string(),
        (None, false) => String::new(),
    };
    let _ = notifier.fire(title, &message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use chrono::NaiveTime;

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn fires_when_event_enabled_and_no_quiet_hours() {
        let cfg = Config::default();
        assert!(should_fire(&cfg, CanonicalEvent::Done, t(14, 0)));
    }

    #[test]
    fn does_not_fire_when_event_disabled() {
        let mut cfg = Config::default();
        cfg.events.subagent_done = false;
        assert!(!should_fire(&cfg, CanonicalEvent::SubagentDone, t(14, 0)));
    }

    #[test]
    fn respects_quiet_hours_crossing_midnight() {
        let mut cfg = Config::default();
        cfg.dnd.enabled = true;
        cfg.dnd.start = "22:00".to_string();
        cfg.dnd.end = "08:00".to_string();
        assert!(!should_fire(&cfg, CanonicalEvent::Done, t(23, 0)));
        assert!(!should_fire(&cfg, CanonicalEvent::Done, t(3, 0)));
        assert!(should_fire(&cfg, CanonicalEvent::Done, t(12, 0)));
    }

    #[test]
    fn handle_notify_calls_the_notifier_when_it_should_fire() {
        let cfg = Config::default();
        let notifier = FakeNotifier::default();
        handle_notify(&cfg, "claude-code", CanonicalEvent::Done, None, None, &notifier, t(12, 0));
        assert_eq!(notifier.calls.borrow().len(), 1);
    }

    #[test]
    fn handle_notify_skips_the_notifier_when_muted_by_quiet_hours() {
        let mut cfg = Config::default();
        cfg.dnd.enabled = true;
        cfg.dnd.start = "00:00".to_string();
        cfg.dnd.end = "23:59".to_string();
        let notifier = FakeNotifier::default();
        handle_notify(&cfg, "claude-code", CanonicalEvent::Done, None, None, &notifier, t(12, 0));
        assert_eq!(notifier.calls.borrow().len(), 0);
    }
}
