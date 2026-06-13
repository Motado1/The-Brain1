//! Human-readable JSON snapshot — a clean, diff-able backup format alongside the binary `.db`.
//!
//! Rows are exported in a stable order so `export → import → export` round-trips to an equal
//! [`Snapshot`] (the idempotency the snapshot test asserts).

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::model::*;
use crate::{repo, Db, Result};

/// A layer assignment row, kept in its stored string form for a lossless round-trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerRow {
    pub entity_id: Id,
    pub layer: String,
}

/// A full database dump.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub entities: Vec<Entity>,
    pub crm: Vec<CrmFacet>,
    pub ledger: Vec<LedgerFacet>,
    pub knowledge: Vec<KnowledgeFacet>,
    pub edges: Vec<Edge>,
    pub activations: Vec<Activation>,
    pub layers: Vec<LayerRow>,
}

/// Read the entire database into a [`Snapshot`], ordered for determinism.
pub fn export(db: &Db) -> Result<Snapshot> {
    let conn = &db.conn;

    let entities = {
        let mut s = conn.prepare("SELECT * FROM entity ORDER BY id")?;
        let rows = s.query_map([], |r| {
            Ok(Entity {
                id: r.get("id")?,
                created_at: r.get("created_at")?,
                updated_at: r.get("updated_at")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let crm = {
        let mut s = conn.prepare("SELECT * FROM crm_facet ORDER BY entity_id")?;
        let rows = s.query_map([], |r| {
            Ok(CrmFacet {
                entity_id: r.get("entity_id")?,
                contact: r.get("contact")?,
                lifecycle_stage: r.get("lifecycle_stage")?,
                session_schedule: r.get("session_schedule")?,
                renewal_date: r.get("renewal_date")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let ledger = {
        let mut s = conn.prepare("SELECT * FROM ledger_facet ORDER BY entity_id")?;
        let rows = s.query_map([], |r| {
            Ok(LedgerFacet {
                entity_id: r.get("entity_id")?,
                amount_cents: r.get("amount_cents")?,
                invoice_status: r.get("invoice_status")?,
                is_expense: r.get("is_expense")?,
                tax_bucket: r.get("tax_bucket")?,
                pacing_target_cents: r.get("pacing_target_cents")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let knowledge = {
        let mut s = conn.prepare("SELECT * FROM knowledge_facet ORDER BY entity_id")?;
        let rows = s.query_map([], |r| {
            Ok(KnowledgeFacet {
                entity_id: r.get("entity_id")?,
                body_md: r.get("body_md")?,
                template_type: r.get("template_type")?,
                review_status: r.get("review_status")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let edges = {
        let mut s = conn.prepare("SELECT * FROM edge ORDER BY id")?;
        let rows = s.query_map([], |r| {
            Ok(Edge {
                id: r.get("id")?,
                source_id: r.get("source_id")?,
                target_id: r.get("target_id")?,
                edge_type: r.get("edge_type")?,
                weight: r.get("weight")?,
                directed: r.get("directed")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let activations = {
        let mut s = conn.prepare("SELECT * FROM activation ORDER BY entity_id")?;
        let rows = s.query_map([], |r| {
            Ok(Activation {
                entity_id: r.get("entity_id")?,
                value: r.get("value")?,
                threshold: r.get("threshold")?,
                last_fired_at: r.get("last_fired_at")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let layers = {
        let mut s = conn.prepare("SELECT entity_id, layer FROM layer_assignment ORDER BY entity_id")?;
        let rows = s.query_map([], |r| {
            Ok(LayerRow {
                entity_id: r.get("entity_id")?,
                layer: r.get("layer")?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    Ok(Snapshot {
        entities,
        crm,
        ledger,
        knowledge,
        edges,
        activations,
        layers,
    })
}

/// Insert a [`Snapshot`] into `db` (intended for an empty database). Runs in one transaction.
pub fn import(db: &mut Db, snap: &Snapshot) -> Result<()> {
    let tx = db.conn.transaction()?;
    for e in &snap.entities {
        repo::insert_entity(&tx, e)?;
    }
    for f in &snap.crm {
        repo::upsert_crm(&tx, f)?;
    }
    for f in &snap.ledger {
        repo::upsert_ledger(&tx, f)?;
    }
    for f in &snap.knowledge {
        repo::upsert_knowledge(&tx, f)?;
    }
    for a in &snap.activations {
        repo::upsert_activation(&tx, a)?;
    }
    for e in &snap.edges {
        repo::insert_edge(&tx, e)?;
    }
    for l in &snap.layers {
        tx.execute(
            "INSERT OR REPLACE INTO layer_assignment (entity_id, layer) VALUES (?1, ?2)",
            params![l.entity_id, l.layer],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Serialize a snapshot to pretty JSON.
pub fn export_json_string(db: &Db) -> Result<String> {
    Ok(serde_json::to_string_pretty(&export(db)?)?)
}

/// Load a snapshot from JSON and import it into `db`.
pub fn import_json_string(db: &mut Db, json: &str) -> Result<()> {
    let snap: Snapshot = serde_json::from_str(json)?;
    import(db, &snap)
}
