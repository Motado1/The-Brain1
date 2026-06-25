//! Deterministic seed generator — builds a layered ANN topology (default 500 entities /
//! 1,200 edges) with mixed facets, for rendering and performance work in later phases.
//!
//! Determinism: identical [`SeedConfig::rng_seed`] ⇒ byte-identical database (same ids, same
//! edges), which is what the seed test asserts. All randomness flows from a single
//! `ChaCha8Rng`; timestamps are derived from a fixed base, never the wall clock.

use std::collections::HashSet;

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::model::*;
use crate::{repo, Db, Error, Result};

/// Configuration for [`seed`].
#[derive(Debug, Clone)]
pub struct SeedConfig {
    pub entities: usize,
    pub edges: usize,
    pub hidden_layers: u8,
    pub rng_seed: u64,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            entities: 500,
            edges: 1200,
            hidden_layers: 3,
            rng_seed: 0x4E42_4531, // "NBE1"
        }
    }
}

const BASE_TS: i64 = 1_700_000_000;

// Sample profile copy for seeded clients. Indexed by entity ordinal `i` (NOT the rng) so the seed
// stays byte-deterministic — adding these rows must not perturb the random stream.
const GOALS: [&str; 4] = [
    "Build strength, lose 5kg",
    "Marathon prep, improve endurance",
    "Post-injury mobility + core",
    "General fitness, tone up",
];
const DIETS: [&str; 4] = [
    "High protein, low carb",
    "Vegetarian, dairy-free",
    "Intermittent fasting 16:8",
    "No restrictions",
];
const INJURIES: [&str; 3] = [
    "Left knee ACL repair 2022",
    "Lower back strain, recurring",
    "Right shoulder impingement",
];

fn rid(rng: &mut ChaCha8Rng) -> String {
    let bytes: [u8; 16] = rng.random();
    uuid::Uuid::from_bytes(bytes).to_string()
}

fn pick<'a>(rng: &mut ChaCha8Rng, slice: &'a [Id]) -> &'a Id {
    &slice[rng.random_range(0..slice.len())]
}

/// Populate `db` with a deterministic layered graph. Existing rows are left untouched
/// (`INSERT OR REPLACE` by fresh ids), but intended to run against an empty database.
pub fn seed(db: &mut Db, cfg: &SeedConfig) -> Result<()> {
    let mut rng = ChaCha8Rng::seed_from_u64(cfg.rng_seed);

    let n_input = cfg.entities * 15 / 100;
    let n_output = cfg.entities * 15 / 100;
    let n_hidden = cfg.entities.saturating_sub(n_input + n_output);

    // Columns: 0 = input, 1..=hidden_layers = hidden, hidden_layers+1 = output.
    let n_cols = cfg.hidden_layers as usize + 2;
    let mut columns: Vec<Vec<Id>> = vec![Vec::new(); n_cols];

    let tx = db.conn.transaction()?;

    // 1) Entities + per-role facets + activation + layer assignment.
    for i in 0..cfg.entities {
        let id = rid(&mut rng);
        let layer = if i < n_input {
            Layer::Input
        } else if i < n_input + n_hidden {
            Layer::Hidden((i % cfg.hidden_layers as usize) as u8)
        } else {
            Layer::Output
        };

        repo::insert_entity(
            &tx,
            &Entity {
                id: id.clone(),
                created_at: BASE_TS + i as i64,
                updated_at: BASE_TS + i as i64,
            },
        )?;
        repo::upsert_activation(
            &tx,
            &Activation {
                entity_id: id.clone(),
                value: rng.random::<f64>(),
                threshold: 0.5,
                last_fired_at: None,
            },
        )?;
        repo::set_layer(&tx, &id, &layer)?;

        match layer {
            // Raw inputs: incoming ledger items or raw text snippets.
            Layer::Input => {
                if rng.random_bool(0.5) {
                    repo::upsert_ledger(
                        &tx,
                        &LedgerFacet {
                            entity_id: id.clone(),
                            amount_cents: rng.random_range(500..50_000),
                            invoice_status: "draft".into(),
                            is_expense: rng.random_bool(0.5),
                            tax_bucket: None,
                            pacing_target_cents: None,
                        },
                    )?;
                } else {
                    repo::upsert_knowledge(
                        &tx,
                        &KnowledgeFacet {
                            entity_id: id.clone(),
                            body_md: format!("# Raw snippet {i}\nUnprocessed inbound note."),
                            template_type: None,
                            review_status: "draft".into(),
                        },
                    )?;
                }
            }
            // Hidden: active clients (CRM), ~40% also carry a knowledge facet (polymorphic).
            Layer::Hidden(_) => {
                let stage = ["lead", "active", "active", "renewal"][rng.random_range(0..4)];
                repo::upsert_crm(
                    &tx,
                    &CrmFacet {
                        entity_id: id.clone(),
                        contact: Some(format!("client-{i}@example.com")),
                        lifecycle_stage: stage.into(),
                        session_schedule: Some("weekly".into()),
                        renewal_date: Some(BASE_TS + rng.random_range(0..90) * 86_400),
                    },
                )?;
                if rng.random_bool(0.4) {
                    repo::upsert_knowledge(
                        &tx,
                        &KnowledgeFacet {
                            entity_id: id.clone(),
                            body_md: format!("# SOP {i}\nLinked research / training note."),
                            template_type: Some("sop".into()),
                            review_status: "reviewed".into(),
                        },
                    )?;
                }
                // Client profile (the "planet" detail). Deterministic from `i`; every 3rd client
                // carries an injury note, so some profiles are partial — exercising the empty-aspect path.
                repo::upsert_profile(
                    &tx,
                    &ProfileFacet {
                        entity_id: id.clone(),
                        fitness_goals: Some(GOALS[i % GOALS.len()].into()),
                        dietary_needs: Some(DIETS[i % DIETS.len()].into()),
                        injury_history: (i % 3 == 0).then(|| INJURIES[i % INJURIES.len()].into()),
                    },
                )?;
            }
            // Output: finalized financial outcomes (income totals).
            Layer::Output => {
                let status = ["paid", "paid", "sent", "overdue"][rng.random_range(0..4)];
                repo::upsert_ledger(
                    &tx,
                    &LedgerFacet {
                        entity_id: id.clone(),
                        amount_cents: rng.random_range(10_000..500_000),
                        invoice_status: status.into(),
                        is_expense: false,
                        tax_bucket: Some("income".into()),
                        pacing_target_cents: Some(rng.random_range(50_000..600_000)),
                    },
                )?;
            }
        }

        columns[layer.column(cfg.hidden_layers) as usize].push(id);
    }

    // 2) Edges: predominantly feed-forward (column c -> c+1), with occasional intra-hidden
    //    reference backlinks. Deduped to hit exactly `cfg.edges`.
    let mut seen: HashSet<(Id, Id)> = HashSet::new();
    let mut made = 0usize;
    let max_attempts = cfg.edges * 50 + 10_000;
    let mut attempts = 0usize;

    while made < cfg.edges && attempts < max_attempts {
        attempts += 1;
        let c = rng.random_range(0..n_cols - 1);
        let intra = c >= 1 && c <= cfg.hidden_layers as usize && rng.random_bool(0.15);
        let (src_col, tgt_col, etype) = if intra {
            (c, c, "reference")
        } else {
            (c, c + 1, "flow")
        };

        if columns[src_col].is_empty() || columns[tgt_col].is_empty() {
            continue;
        }
        let src = pick(&mut rng, &columns[src_col]).clone();
        let tgt = pick(&mut rng, &columns[tgt_col]).clone();
        if src == tgt || !seen.insert((src.clone(), tgt.clone())) {
            continue;
        }

        repo::insert_edge(
            &tx,
            &Edge {
                id: rid(&mut rng),
                source_id: src,
                target_id: tgt,
                edge_type: etype.into(),
                weight: rng.random_range(0.1..1.0),
                directed: !intra,
            },
        )?;
        made += 1;
    }

    if made < cfg.edges {
        return Err(Error::Msg(format!(
            "seed could only place {made}/{} edges for this configuration",
            cfg.edges
        )));
    }

    tx.commit()?;
    Ok(())
}
