//! Per-entity "data anatomy" derived from the DB snapshot: the sub-records that get woven directly
//! into a neuron's dendrite tree (sessions as beads along branches; packages + cross-domain research
//! proxies as terminal twigs) — never as detached orbiting satellites. Pure + unit-tested headless;
//! the geometry/scene layer turns this into beads and twigs.

use std::collections::HashMap;

use nbe_data::snapshot::Snapshot;

/// Outcome of a logged PT session — drives the bead's colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SessionStatus {
    Completed,
    NoShow,
    Cancelled,
}

impl SessionStatus {
    fn parse(s: &str) -> Self {
        match s {
            "no_show" => Self::NoShow,
            "cancelled" => Self::Cancelled,
            _ => Self::Completed,
        }
    }
}

/// A package shown as a terminal twig — active (bright) vs depleted (dim).
pub(crate) struct PackageTwig {
    pub(crate) active: bool,
}

/// Everything that hangs off one neuron's branches.
#[derive(Default)]
pub(crate) struct Anatomy {
    /// Session outcomes in chronological order (beads strung along a branch).
    pub(crate) sessions: Vec<SessionStatus>,
    /// Packages (terminal twigs).
    pub(crate) packages: Vec<PackageTwig>,
    /// Cross-domain research links (terminal proxy twigs): for a client, notes that mention it;
    /// for a note, the topics/clients it links out to.
    pub(crate) research_proxies: usize,
}

/// Build per-entity anatomy from the snapshot. Keyed by entity id; only entities with sub-records
/// appear. Sessions are sorted chronologically so the bead chain reads oldest→newest.
pub(crate) fn build_anatomy(snap: &Snapshot) -> HashMap<String, Anatomy> {
    let mut map: HashMap<String, Anatomy> = HashMap::new();

    let mut sessions = snap.sessions.clone();
    sessions.sort_by_key(|s| s.occurred_at);
    for s in &sessions {
        map.entry(s.client_id.clone())
            .or_default()
            .sessions
            .push(SessionStatus::parse(&s.status));
    }

    for p in &snap.packages {
        map.entry(p.client_id.clone())
            .or_default()
            .packages
            .push(PackageTwig { active: p.active });
    }

    // Cross-domain research proxies: a `mentions` edge (note→client) credits the client; a `topic`
    // edge (note→topic) credits the note. Either way the *source/target* gains a proxy twig.
    for e in &snap.edges {
        match e.edge_type.as_str() {
            "mentions" => *proxy(&mut map, &e.target_id) += 1,
            "topic" => *proxy(&mut map, &e.source_id) += 1,
            _ => {}
        }
    }

    map
}

fn proxy<'a>(map: &'a mut HashMap<String, Anatomy>, id: &str) -> &'a mut usize {
    &mut map.entry(id.to_string()).or_default().research_proxies
}

#[cfg(test)]
mod tests {
    use super::*;
    use nbe_data::model::{Edge, Package, Session};

    fn session(client: &str, at: i64, status: &str) -> Session {
        Session {
            id: format!("s{at}"),
            client_id: client.into(),
            package_id: None,
            occurred_at: at,
            status: status.into(),
            source: "manual".into(),
            external_id: None,
            note: None,
        }
    }

    #[test]
    fn anatomy_groups_sessions_packages_and_proxies() {
        let mut snap = Snapshot::default();
        snap.sessions = vec![
            session("alice", 30, "completed"),
            session("alice", 10, "no_show"),
            session("alice", 20, "completed"),
        ];
        snap.packages = vec![Package {
            id: "p1".into(),
            client_id: "alice".into(),
            kind: "PT10".into(),
            total_sessions: 10,
            price_cents: 0,
            purchased_at: 0,
            active: true,
        }];
        snap.edges = vec![Edge {
            id: "e1".into(),
            source_id: "note1".into(),
            target_id: "alice".into(),
            edge_type: "mentions".into(),
            weight: 1.0,
            directed: true,
        }];

        let a = build_anatomy(&snap);
        let alice = a.get("alice").unwrap();
        // chronological order: 10 (no_show), 20 (completed), 30 (completed)
        assert_eq!(
            alice.sessions,
            vec![SessionStatus::NoShow, SessionStatus::Completed, SessionStatus::Completed]
        );
        assert_eq!(alice.packages.len(), 1);
        assert!(alice.packages[0].active);
        assert_eq!(alice.research_proxies, 1, "one note mentions alice");
        // The mentioning note isn't a client, so it has no sessions but exists if it had topics.
        assert!(a.get("note1").is_none(), "note1 has no topic edges here");
    }
}
