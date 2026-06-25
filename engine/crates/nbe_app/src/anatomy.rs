//! Per-entity "data anatomy" derived from the DB snapshot: a fixed-order list of **aspects** — the
//! handful of profile/knowledge facts that become the small "planet" nodes at a sun's dendrite tips.
//! Pure + unit-tested headless; the scene layer (`embed_planets`) turns each aspect into a billboard
//! at a branch tip. Exactly five aspects per sun, in a stable order, so the planet layout is fixed
//! whether or not a given fact is filled in (`present = false` → a dim placeholder planet).

use std::collections::{HashMap, HashSet};

use nbe_data::snapshot::Snapshot;

/// The five facts shown as planets for each kind of sun, in fixed render order. Clients (CRM suns)
/// use the first five; knowledge notes (Research suns) use the last five.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AspectKind {
    // Client sun, in this order.
    Goals,
    Diet,
    Injury,
    Schedule,
    Contact,
    // Knowledge sun, in this order.
    Body,
    Status,
    Mentions,
    Topics,
    References,
}

impl AspectKind {
    /// All ten aspect kinds in fixed render order (client five, then knowledge five).
    pub(crate) const ALL: [AspectKind; 10] = [
        AspectKind::Goals,
        AspectKind::Diet,
        AspectKind::Injury,
        AspectKind::Schedule,
        AspectKind::Contact,
        AspectKind::Body,
        AspectKind::Status,
        AspectKind::Mentions,
        AspectKind::Topics,
        AspectKind::References,
    ];

    /// Stable 0..9 index, used to key the shared planet-material palette.
    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|&k| k == self).unwrap()
    }
}

/// One planet's worth of data: which fact, a short label, a 0..1 intensity (drives planet brightness/
/// scale), and whether the fact is actually filled in (absent → a dim placeholder so layout is stable).
#[derive(Clone)]
pub(crate) struct Aspect {
    pub(crate) kind: AspectKind,
    pub(crate) label: String,
    pub(crate) value: f32,
    pub(crate) present: bool,
}

/// Exactly five aspects per sun, in fixed order.
#[derive(Default, Clone)]
pub(crate) struct Anatomy {
    pub(crate) aspects: Vec<Aspect>,
}

/// A free-text fact: present iff the trimmed text is non-empty; full intensity when present.
fn text_aspect(kind: AspectKind, label: &str, text: Option<&str>) -> Aspect {
    let present = text.map(|t| !t.trim().is_empty()).unwrap_or(false);
    Aspect { kind, label: label.to_string(), value: if present { 1.0 } else { 0.0 }, present }
}

/// A countable fact (edges of a given type): intensity ramps `n/scale` clamped to 1; present iff `n>0`.
fn count_aspect(kind: AspectKind, label: &str, n: usize, scale: f32) -> Aspect {
    Aspect {
        kind,
        label: label.to_string(),
        value: (n as f32 / scale).clamp(0.0, 1.0),
        present: n > 0,
    }
}

/// Build per-sun anatomy from the snapshot. Keyed by entity id. Every client (CRM facet) and every
/// non-client knowledge note gets exactly five aspects in fixed order — present or not — so the planet
/// layout never shifts. A client takes precedence over a knowledge facet sharing the same id.
pub(crate) fn build_anatomy(snap: &Snapshot) -> HashMap<String, Anatomy> {
    // ---- indices -----------------------------------------------------------------------------
    let profile_by: HashMap<&str, &nbe_data::model::ProfileFacet> =
        snap.profile.iter().map(|p| (p.entity_id.as_str(), p)).collect();
    let client_ids: HashSet<&str> = snap.crm.iter().map(|c| c.entity_id.as_str()).collect();

    // Weekly cadence per client = sum of its slot cadences.
    let mut cadence: HashMap<&str, f32> = HashMap::new();
    for s in &snap.slots {
        *cadence.entry(s.client_id.as_str()).or_insert(0.0) += s.cadence as f32;
    }

    // Edge tallies: a note's outgoing mentions/topics (by source), and references pointing at it (by
    // target).
    let mut mentions: HashMap<&str, usize> = HashMap::new();
    let mut topics: HashMap<&str, usize> = HashMap::new();
    let mut references: HashMap<&str, usize> = HashMap::new();
    for e in &snap.edges {
        match e.edge_type.as_str() {
            "mentions" => *mentions.entry(e.source_id.as_str()).or_insert(0) += 1,
            "topic" => *topics.entry(e.source_id.as_str()).or_insert(0) += 1,
            "reference" => *references.entry(e.target_id.as_str()).or_insert(0) += 1,
            _ => {}
        }
    }

    let mut map: HashMap<String, Anatomy> = HashMap::new();

    // ---- client suns -------------------------------------------------------------------------
    for c in &snap.crm {
        let id = c.entity_id.as_str();
        let prof = profile_by.get(id).copied();
        let cad = cadence.get(id).copied().unwrap_or(0.0);
        let contact_present = c.contact.as_deref().map(|t| !t.trim().is_empty()).unwrap_or(false);
        let contact_value = match c.lifecycle_stage.as_str() {
            "renewal" => 1.0,
            "active" => 0.6,
            "lead" => 0.4,
            _ => 0.2,
        };
        let aspects = vec![
            text_aspect(AspectKind::Goals, "Goals", prof.and_then(|p| p.fitness_goals.as_deref())),
            text_aspect(AspectKind::Diet, "Diet", prof.and_then(|p| p.dietary_needs.as_deref())),
            text_aspect(
                AspectKind::Injury,
                "Injury",
                prof.and_then(|p| p.injury_history.as_deref()),
            ),
            Aspect {
                kind: AspectKind::Schedule,
                label: format!("{cad:.1}x/wk"),
                value: (cad / 3.0).clamp(0.0, 1.0),
                present: cad > 0.0,
            },
            Aspect {
                kind: AspectKind::Contact,
                label: "Contact".to_string(),
                value: contact_value,
                present: contact_present,
            },
        ];
        map.insert(id.to_string(), Anatomy { aspects });
    }

    // ---- knowledge suns (notes that aren't clients) ------------------------------------------
    for k in &snap.knowledge {
        let id = k.entity_id.as_str();
        if client_ids.contains(id) {
            continue; // client takes precedence
        }
        let body_len = k.body_md.chars().count();
        let status_value = match k.review_status.as_str() {
            "reviewed" => 1.0,
            "draft" => 0.4,
            "archived" => 0.15,
            _ => 0.3,
        };
        let aspects = vec![
            Aspect {
                kind: AspectKind::Body,
                label: "Body".to_string(),
                value: (body_len as f32 / 400.0).clamp(0.0, 1.0),
                present: body_len > 0,
            },
            Aspect {
                kind: AspectKind::Status,
                label: "Status".to_string(),
                value: status_value,
                present: true,
            },
            count_aspect(
                AspectKind::Mentions,
                "Mentions",
                mentions.get(id).copied().unwrap_or(0),
                5.0,
            ),
            count_aspect(AspectKind::Topics, "Topics", topics.get(id).copied().unwrap_or(0), 5.0),
            count_aspect(
                AspectKind::References,
                "References",
                references.get(id).copied().unwrap_or(0),
                8.0,
            ),
        ];
        map.insert(id.to_string(), Anatomy { aspects });
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use nbe_data::model::{CrmFacet, Edge, KnowledgeFacet, ProfileFacet, Slot};

    fn crm(id: &str, stage: &str, contact: Option<&str>) -> CrmFacet {
        CrmFacet {
            entity_id: id.into(),
            contact: contact.map(Into::into),
            lifecycle_stage: stage.into(),
            session_schedule: None,
            renewal_date: None,
        }
    }

    fn slot(client: &str, cadence: f64) -> Slot {
        Slot { id: format!("sl-{client}-{cadence}"), client_id: client.into(), weekday: 0, start_min: 0, duration_min: 60, cadence }
    }

    fn knowledge(id: &str, body: &str, status: &str) -> KnowledgeFacet {
        KnowledgeFacet {
            entity_id: id.into(),
            body_md: body.into(),
            template_type: None,
            review_status: status.into(),
        }
    }

    fn edge(src: &str, tgt: &str, ty: &str) -> Edge {
        Edge { id: format!("{src}-{tgt}-{ty}"), source_id: src.into(), target_id: tgt.into(), edge_type: ty.into(), weight: 1.0, directed: true }
    }

    #[test]
    fn client_emits_five_aspects_in_fixed_order_with_present_flags() {
        let mut snap = Snapshot::default();
        snap.crm = vec![crm("alice", "renewal", Some("a@b.com"))];
        snap.profile = vec![ProfileFacet {
            entity_id: "alice".into(),
            fitness_goals: Some("lose 5kg".into()),
            dietary_needs: Some("  ".into()), // whitespace → absent
            injury_history: None,
        }];
        snap.slots = vec![slot("alice", 1.5), slot("alice", 0.5)]; // cadence 2.0/wk

        let a = build_anatomy(&snap);
        let alice = a.get("alice").unwrap();
        let kinds: Vec<AspectKind> = alice.aspects.iter().map(|x| x.kind).collect();
        assert_eq!(
            kinds,
            vec![
                AspectKind::Goals,
                AspectKind::Diet,
                AspectKind::Injury,
                AspectKind::Schedule,
                AspectKind::Contact,
            ]
        );
        assert!(alice.aspects[0].present, "goals filled");
        assert!(!alice.aspects[1].present, "whitespace diet absent");
        assert!(!alice.aspects[2].present, "no injury");
        assert!(alice.aspects[3].present, "has cadence");
        assert_eq!(alice.aspects[3].label, "2.0x/wk");
        assert!((alice.aspects[3].value - 2.0 / 3.0).abs() < 1e-5);
        assert!(alice.aspects[4].present, "has contact");
        assert!((alice.aspects[4].value - 1.0).abs() < 1e-5, "renewal lifecycle");
        // All values within bounds.
        assert!(alice.aspects.iter().all(|x| (0.0..=1.0).contains(&x.value)));
    }

    #[test]
    fn empty_client_still_emits_five_absent_aspects() {
        let mut snap = Snapshot::default();
        snap.crm = vec![crm("bob", "lead", None)];
        let a = build_anatomy(&snap);
        let bob = a.get("bob").unwrap();
        assert_eq!(bob.aspects.len(), 5);
        // Goals/Diet/Injury/Schedule/Contact all absent (no profile, no slots, no contact).
        assert!(bob.aspects.iter().all(|x| !x.present));
        assert_eq!(bob.aspects[3].label, "0.0x/wk");
    }

    #[test]
    fn knowledge_counts_edges_by_direction() {
        let mut snap = Snapshot::default();
        snap.knowledge = vec![knowledge("note1", "hello world", "reviewed")];
        snap.edges = vec![
            edge("note1", "alice", "mentions"), // note1 mentions (by source)
            edge("note1", "topicX", "topic"),   // note1 → topic (by source)
            edge("note1", "topicY", "topic"),
            edge("other", "note1", "reference"), // references point at note1 (by target)
        ];
        let a = build_anatomy(&snap);
        let n = a.get("note1").unwrap();
        let kinds: Vec<AspectKind> = n.aspects.iter().map(|x| x.kind).collect();
        assert_eq!(
            kinds,
            vec![
                AspectKind::Body,
                AspectKind::Status,
                AspectKind::Mentions,
                AspectKind::Topics,
                AspectKind::References,
            ]
        );
        assert!(n.aspects[0].present, "has body");
        assert!((n.aspects[1].value - 1.0).abs() < 1e-5, "reviewed");
        assert_eq!(n.aspects[2].value, 1.0 / 5.0, "1 mention / scale 5");
        assert_eq!(n.aspects[3].value, 2.0 / 5.0, "2 topics / scale 5");
        assert_eq!(n.aspects[4].value, 1.0 / 8.0, "1 reference / scale 8");
    }

    #[test]
    fn client_takes_precedence_over_knowledge_facet_on_same_id() {
        let mut snap = Snapshot::default();
        snap.crm = vec![crm("dual", "active", Some("x"))];
        snap.knowledge = vec![knowledge("dual", "body", "draft")];
        let a = build_anatomy(&snap);
        let dual = a.get("dual").unwrap();
        // Client wins → first aspect is Goals (client order), not Body.
        assert_eq!(dual.aspects[0].kind, AspectKind::Goals);
    }
}
