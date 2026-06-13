//! Match calendar events to clients by name, so a "James Bywater" event becomes a logged
//! session for that client.

use crate::ics::CalendarEvent;

/// A calendar event resolved to a specific client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedSession {
    pub client_id: String,
    pub occurred_at: i64,
    pub uid: String,
    pub summary: String,
}

/// Match each event to at most one client whose name appears (case-insensitive) in the event
/// summary. When several clients match, the longest (most specific) name wins. Events with no
/// match are dropped (the caller can report `events.len() - matched.len()` as unmatched).
pub fn match_events(
    events: &[CalendarEvent],
    clients: &[(String, String)],
) -> Vec<MatchedSession> {
    let mut out = Vec::new();
    for e in events {
        let summary = e.summary.to_lowercase();
        let best = clients
            .iter()
            .filter(|(_, name)| {
                let n = name.trim().to_lowercase();
                !n.is_empty() && summary.contains(&n)
            })
            .max_by_key(|(_, name)| name.trim().len());
        if let Some((id, _)) = best {
            out.push(MatchedSession {
                client_id: id.clone(),
                occurred_at: e.start,
                uid: e.uid.clone(),
                summary: e.summary.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(uid: &str, summary: &str) -> CalendarEvent {
        CalendarEvent {
            uid: uid.into(),
            summary: summary.into(),
            start: 1_700_000_000,
            end: None,
        }
    }

    #[test]
    fn matches_by_name_case_insensitive() {
        let clients = vec![("c1".to_string(), "James Bywater".to_string())];
        let m = match_events(&[ev("u1", "james bywater 9/10")], &clients);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].client_id, "c1");
    }

    #[test]
    fn unmatched_events_are_dropped() {
        let clients = vec![("c1".to_string(), "James Bywater".to_string())];
        let m = match_events(&[ev("u1", "Dentist appointment")], &clients);
        assert!(m.is_empty());
    }

    #[test]
    fn longest_name_wins_when_ambiguous() {
        let clients = vec![
            ("c1".to_string(), "James".to_string()),
            ("c2".to_string(), "James Bywater".to_string()),
        ];
        let m = match_events(&[ev("u1", "James Bywater PT")], &clients);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].client_id, "c2", "most specific match");
    }
}
