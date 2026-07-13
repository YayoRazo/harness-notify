use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalEvent {
    Done,
    Attention,
    SubagentDone,
}

impl CanonicalEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            CanonicalEvent::Done => "done",
            CanonicalEvent::Attention => "attention",
            CanonicalEvent::SubagentDone => "subagent-done",
        }
    }
}

impl FromStr for CanonicalEvent {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "done" => Ok(CanonicalEvent::Done),
            "attention" => Ok(CanonicalEvent::Attention),
            "subagent-done" => Ok(CanonicalEvent::SubagentDone),
            other => Err(format!("unknown event: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn round_trips_through_str() {
        for e in [CanonicalEvent::Done, CanonicalEvent::Attention, CanonicalEvent::SubagentDone] {
            let s = e.as_str();
            assert_eq!(CanonicalEvent::from_str(s).unwrap(), e);
        }
    }

    #[test]
    fn rejects_unknown_event_name() {
        assert!(CanonicalEvent::from_str("bogus").is_err());
    }
}
