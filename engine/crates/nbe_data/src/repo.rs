//! Repository layer — CRUD and queries over a [`rusqlite::Connection`].
//!
//! These are free functions taking `&Connection` so they work equally on a [`crate::Db`]'s
//! connection and on a `Transaction` (used by the seed/import paths).

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::model::*;
use crate::Result;

// ---- row mappers ------------------------------------------------------------------------

fn entity_from_row(row: &Row) -> rusqlite::Result<Entity> {
    Ok(Entity {
        id: row.get("id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn crm_from_row(row: &Row) -> rusqlite::Result<CrmFacet> {
    Ok(CrmFacet {
        entity_id: row.get("entity_id")?,
        contact: row.get("contact")?,
        lifecycle_stage: row.get("lifecycle_stage")?,
        session_schedule: row.get("session_schedule")?,
        renewal_date: row.get("renewal_date")?,
    })
}

fn ledger_from_row(row: &Row) -> rusqlite::Result<LedgerFacet> {
    Ok(LedgerFacet {
        entity_id: row.get("entity_id")?,
        amount_cents: row.get("amount_cents")?,
        invoice_status: row.get("invoice_status")?,
        is_expense: row.get("is_expense")?,
        tax_bucket: row.get("tax_bucket")?,
        pacing_target_cents: row.get("pacing_target_cents")?,
    })
}

fn knowledge_from_row(row: &Row) -> rusqlite::Result<KnowledgeFacet> {
    Ok(KnowledgeFacet {
        entity_id: row.get("entity_id")?,
        body_md: row.get("body_md")?,
        template_type: row.get("template_type")?,
        review_status: row.get("review_status")?,
    })
}

fn edge_from_row(row: &Row) -> rusqlite::Result<Edge> {
    Ok(Edge {
        id: row.get("id")?,
        source_id: row.get("source_id")?,
        target_id: row.get("target_id")?,
        edge_type: row.get("edge_type")?,
        weight: row.get("weight")?,
        directed: row.get("directed")?,
    })
}

fn activation_from_row(row: &Row) -> rusqlite::Result<Activation> {
    Ok(Activation {
        entity_id: row.get("entity_id")?,
        value: row.get("value")?,
        threshold: row.get("threshold")?,
        last_fired_at: row.get("last_fired_at")?,
    })
}

// ---- inserts / upserts ------------------------------------------------------------------

pub fn insert_entity(conn: &Connection, e: &Entity) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO entity (id, created_at, updated_at) VALUES (?1, ?2, ?3)",
        params![e.id, e.created_at, e.updated_at],
    )?;
    Ok(())
}

pub fn upsert_crm(conn: &Connection, f: &CrmFacet) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO crm_facet
            (entity_id, contact, lifecycle_stage, session_schedule, renewal_date)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            f.entity_id,
            f.contact,
            f.lifecycle_stage,
            f.session_schedule,
            f.renewal_date
        ],
    )?;
    Ok(())
}

pub fn upsert_ledger(conn: &Connection, f: &LedgerFacet) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO ledger_facet
            (entity_id, amount_cents, invoice_status, is_expense, tax_bucket, pacing_target_cents)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            f.entity_id,
            f.amount_cents,
            f.invoice_status,
            f.is_expense,
            f.tax_bucket,
            f.pacing_target_cents
        ],
    )?;
    Ok(())
}

pub fn upsert_knowledge(conn: &Connection, f: &KnowledgeFacet) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO knowledge_facet
            (entity_id, body_md, template_type, review_status)
         VALUES (?1, ?2, ?3, ?4)",
        params![f.entity_id, f.body_md, f.template_type, f.review_status],
    )?;
    Ok(())
}

pub fn upsert_activation(conn: &Connection, a: &Activation) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO activation (entity_id, value, threshold, last_fired_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![a.entity_id, a.value, a.threshold, a.last_fired_at],
    )?;
    Ok(())
}

pub fn set_layer(conn: &Connection, entity_id: &str, layer: &Layer) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO layer_assignment (entity_id, layer) VALUES (?1, ?2)",
        params![entity_id, layer.to_db()],
    )?;
    Ok(())
}

pub fn insert_edge(conn: &Connection, e: &Edge) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO edge (id, source_id, target_id, edge_type, weight, directed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            e.id,
            e.source_id,
            e.target_id,
            e.edge_type,
            e.weight,
            e.directed
        ],
    )?;
    Ok(())
}

// ---- deletes ----------------------------------------------------------------------------

/// Delete an entity and everything that cascades from it — facets, edges (either direction),
/// activation, layer, and any PT packages/sessions/slots — via the schema's `ON DELETE CASCADE`
/// foreign keys. Returns `true` when a row actually existed.
pub fn delete_entity(conn: &Connection, id: &str) -> Result<bool> {
    let changed = conn.execute("DELETE FROM entity WHERE id = ?1", params![id])?;
    Ok(changed > 0)
}

/// Remove directed edges from `source` to `target`, optionally restricted to one `edge_type`.
/// Returns the number of edges removed.
pub fn delete_edges_between(
    conn: &Connection,
    source: &str,
    target: &str,
    edge_type: Option<&str>,
) -> Result<usize> {
    let changed = match edge_type {
        Some(t) => conn.execute(
            "DELETE FROM edge WHERE source_id = ?1 AND target_id = ?2 AND edge_type = ?3",
            params![source, target, t],
        )?,
        None => conn.execute(
            "DELETE FROM edge WHERE source_id = ?1 AND target_id = ?2",
            params![source, target],
        )?,
    };
    Ok(changed)
}

// ---- single-row reads -------------------------------------------------------------------

pub fn get_entity(conn: &Connection, id: &str) -> Result<Option<Entity>> {
    Ok(conn
        .query_row("SELECT * FROM entity WHERE id = ?1", params![id], entity_from_row)
        .optional()?)
}

pub fn get_crm(conn: &Connection, id: &str) -> Result<Option<CrmFacet>> {
    Ok(conn
        .query_row(
            "SELECT * FROM crm_facet WHERE entity_id = ?1",
            params![id],
            crm_from_row,
        )
        .optional()?)
}

pub fn get_ledger(conn: &Connection, id: &str) -> Result<Option<LedgerFacet>> {
    Ok(conn
        .query_row(
            "SELECT * FROM ledger_facet WHERE entity_id = ?1",
            params![id],
            ledger_from_row,
        )
        .optional()?)
}

pub fn get_knowledge(conn: &Connection, id: &str) -> Result<Option<KnowledgeFacet>> {
    Ok(conn
        .query_row(
            "SELECT * FROM knowledge_facet WHERE entity_id = ?1",
            params![id],
            knowledge_from_row,
        )
        .optional()?)
}

pub fn get_activation(conn: &Connection, id: &str) -> Result<Option<Activation>> {
    Ok(conn
        .query_row(
            "SELECT * FROM activation WHERE entity_id = ?1",
            params![id],
            activation_from_row,
        )
        .optional()?)
}

pub fn get_layer(conn: &Connection, id: &str) -> Result<Option<Layer>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT layer FROM layer_assignment WHERE entity_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(raw.as_deref().and_then(Layer::from_db))
}

/// The polymorphic view: an entity plus every facet it currently carries.
pub fn entity_with_facets(conn: &Connection, id: &str) -> Result<Option<EntityWithFacets>> {
    let Some(entity) = get_entity(conn, id)? else {
        return Ok(None);
    };
    Ok(Some(EntityWithFacets {
        entity: Some(entity),
        crm: get_crm(conn, id)?,
        ledger: get_ledger(conn, id)?,
        knowledge: get_knowledge(conn, id)?,
        activation: get_activation(conn, id)?,
        layer: get_layer(conn, id)?,
    }))
}

// ---- multi-row queries ------------------------------------------------------------------

/// Incoming edges (backlinks): every edge whose target is `id`.
pub fn backlinks(conn: &Connection, id: &str) -> Result<Vec<Edge>> {
    let mut stmt =
        conn.prepare("SELECT * FROM edge WHERE target_id = ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![id], edge_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Outgoing edges: every edge whose source is `id`.
pub fn outgoing(conn: &Connection, id: &str) -> Result<Vec<Edge>> {
    let mut stmt =
        conn.prepare("SELECT * FROM edge WHERE source_id = ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![id], edge_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Entity ids assigned to a given layer (ordered for determinism).
pub fn ids_by_layer(conn: &Connection, layer: &Layer) -> Result<Vec<Id>> {
    let mut stmt = conn
        .prepare("SELECT entity_id FROM layer_assignment WHERE layer = ?1 ORDER BY entity_id")?;
    let rows = stmt.query_map(params![layer.to_db()], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn count_entities(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM entity", [], |row| row.get(0))?)
}

pub fn count_edges(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM edge", [], |row| row.get(0))?)
}

/// Aggregated financial roll-up (feeds the Output layer). `paid + outstanding == total`.
pub fn ledger_rollup(conn: &Connection) -> Result<LedgerRollup> {
    Ok(conn.query_row("SELECT * FROM v_ledger_rollup", [], |row| {
        Ok(LedgerRollup {
            total_cents: row.get("total_cents")?,
            paid_cents: row.get("paid_cents")?,
            outstanding_cents: row.get("outstanding_cents")?,
            income_cents: row.get("income_cents")?,
            expense_cents: row.get("expense_cents")?,
        })
    })?)
}

/// Every entity id, ordered (deterministic).
pub fn all_entity_ids(conn: &Connection) -> Result<Vec<Id>> {
    let mut stmt = conn.prepare("SELECT id FROM entity ORDER BY id")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Entity ids beginning with `prefix` (for short-id resolution, git-style). Wildcards stripped.
pub fn find_ids_by_prefix(conn: &Connection, prefix: &str) -> Result<Vec<Id>> {
    let cleaned: String = prefix.chars().filter(|c| *c != '%' && *c != '_').collect();
    let pattern = format!("{cleaned}%");
    let mut stmt = conn.prepare("SELECT id FROM entity WHERE id LIKE ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![pattern], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_crm(conn: &Connection) -> Result<Vec<CrmFacet>> {
    let mut stmt = conn.prepare("SELECT * FROM crm_facet ORDER BY entity_id")?;
    let rows = stmt.query_map([], crm_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_ledger(conn: &Connection) -> Result<Vec<LedgerFacet>> {
    let mut stmt = conn.prepare("SELECT * FROM ledger_facet ORDER BY entity_id")?;
    let rows = stmt.query_map([], ledger_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_knowledge(conn: &Connection) -> Result<Vec<KnowledgeFacet>> {
    let mut stmt = conn.prepare("SELECT * FROM knowledge_facet ORDER BY entity_id")?;
    let rows = stmt.query_map([], knowledge_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Sum of ledger amounts grouped by tax bucket, ordered by bucket.
pub fn ledger_by_bucket(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(tax_bucket, '(none)') AS bucket, SUM(amount_cents) AS total
         FROM ledger_facet GROUP BY bucket ORDER BY bucket",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// ---- packages / sessions / slots / config (v2) -----------------------------------------

fn package_from_row(row: &Row) -> rusqlite::Result<Package> {
    Ok(Package {
        id: row.get("id")?,
        client_id: row.get("client_id")?,
        kind: row.get("kind")?,
        total_sessions: row.get("total_sessions")?,
        price_cents: row.get("price_cents")?,
        purchased_at: row.get("purchased_at")?,
        active: row.get("active")?,
    })
}

fn session_from_row(row: &Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get("id")?,
        client_id: row.get("client_id")?,
        package_id: row.get("package_id")?,
        occurred_at: row.get("occurred_at")?,
        status: row.get("status")?,
        source: row.get("source")?,
        external_id: row.get("external_id")?,
        note: row.get("note")?,
    })
}

fn slot_from_row(row: &Row) -> rusqlite::Result<Slot> {
    Ok(Slot {
        id: row.get("id")?,
        client_id: row.get("client_id")?,
        weekday: row.get("weekday")?,
        start_min: row.get("start_min")?,
        duration_min: row.get("duration_min")?,
        cadence: row.get("cadence")?,
    })
}

pub fn insert_package(conn: &Connection, p: &Package) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO package
            (id, client_id, kind, total_sessions, price_cents, purchased_at, active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            p.id,
            p.client_id,
            p.kind,
            p.total_sessions,
            p.price_cents,
            p.purchased_at,
            p.active
        ],
    )?;
    Ok(())
}

/// Mark every package for a client inactive (called before a renewal activates a new one).
pub fn deactivate_packages(conn: &Connection, client_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE package SET active = 0 WHERE client_id = ?1",
        params![client_id],
    )?;
    Ok(())
}

/// The most recent package for a client (active or not) — used to restore the previous package
/// after an erroneous renewal is deleted.
pub fn latest_package(conn: &Connection, client_id: &str) -> Result<Option<Package>> {
    Ok(conn
        .query_row(
            "SELECT * FROM package WHERE client_id = ?1 ORDER BY purchased_at DESC, id DESC LIMIT 1",
            params![client_id],
            package_from_row,
        )
        .optional()?)
}

pub fn set_package_active(conn: &Connection, id: &str, active: bool) -> Result<()> {
    conn.execute(
        "UPDATE package SET active = ?2 WHERE id = ?1",
        params![id, active],
    )?;
    Ok(())
}

pub fn active_package(conn: &Connection, client_id: &str) -> Result<Option<Package>> {
    Ok(conn
        .query_row(
            "SELECT * FROM package WHERE client_id = ?1 AND active = 1
             ORDER BY purchased_at DESC LIMIT 1",
            params![client_id],
            package_from_row,
        )
        .optional()?)
}

pub fn active_packages(conn: &Connection) -> Result<Vec<Package>> {
    let mut stmt =
        conn.prepare("SELECT * FROM package WHERE active = 1 ORDER BY client_id")?;
    let rows = stmt.query_map([], package_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_package(conn: &Connection, id: &str) -> Result<Option<Package>> {
    Ok(conn
        .query_row("SELECT * FROM package WHERE id = ?1", params![id], package_from_row)
        .optional()?)
}

pub fn delete_package(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn.execute("DELETE FROM package WHERE id = ?1", params![id])? > 0)
}

/// Package ids beginning with `prefix` (for short-id resolution).
pub fn find_package_ids_by_prefix(conn: &Connection, prefix: &str) -> Result<Vec<Id>> {
    let cleaned: String = prefix.chars().filter(|c| *c != '%' && *c != '_').collect();
    let pattern = format!("{cleaned}%");
    let mut stmt = conn.prepare("SELECT id FROM package WHERE id LIKE ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![pattern], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_packages(conn: &Connection) -> Result<Vec<Package>> {
    let mut stmt = conn.prepare("SELECT * FROM package ORDER BY id")?;
    let rows = stmt.query_map([], package_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Insert a session, ignoring duplicates by `external_id` (idempotent calendar sync).
/// Returns `true` when a new row was actually inserted.
pub fn insert_session(conn: &Connection, s: &Session) -> Result<bool> {
    let changed = conn.execute(
        "INSERT OR IGNORE INTO session
            (id, client_id, package_id, occurred_at, status, source, external_id, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            s.id,
            s.client_id,
            s.package_id,
            s.occurred_at,
            s.status,
            s.source,
            s.external_id,
            s.note
        ],
    )?;
    Ok(changed > 0)
}

pub fn get_session(conn: &Connection, id: &str) -> Result<Option<Session>> {
    Ok(conn
        .query_row("SELECT * FROM session WHERE id = ?1", params![id], session_from_row)
        .optional()?)
}

/// Overwrite an existing session row (used by edits, unlike the idempotent `insert_session`).
pub fn replace_session(conn: &Connection, s: &Session) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO session
            (id, client_id, package_id, occurred_at, status, source, external_id, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            s.id,
            s.client_id,
            s.package_id,
            s.occurred_at,
            s.status,
            s.source,
            s.external_id,
            s.note
        ],
    )?;
    Ok(())
}

pub fn delete_session(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn.execute("DELETE FROM session WHERE id = ?1", params![id])? > 0)
}

/// Session ids beginning with `prefix` (for short-id resolution, like entities).
pub fn find_session_ids_by_prefix(conn: &Connection, prefix: &str) -> Result<Vec<Id>> {
    let cleaned: String = prefix.chars().filter(|c| *c != '%' && *c != '_').collect();
    let pattern = format!("{cleaned}%");
    let mut stmt = conn.prepare("SELECT id FROM session WHERE id LIKE ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![pattern], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// All sessions for one client, most recent first.
pub fn list_sessions_for(conn: &Connection, client_id: &str) -> Result<Vec<Session>> {
    let mut stmt =
        conn.prepare("SELECT * FROM session WHERE client_id = ?1 ORDER BY occurred_at DESC, id")?;
    let rows = stmt.query_map(params![client_id], session_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn sessions_completed(conn: &Connection, package_id: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM session WHERE package_id = ?1 AND status = 'completed'",
        params![package_id],
        |r| r.get(0),
    )?)
}

pub fn list_sessions(conn: &Connection) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare("SELECT * FROM session ORDER BY id")?;
    let rows = stmt.query_map([], session_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn insert_slot(conn: &Connection, s: &Slot) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO slot
            (id, client_id, weekday, start_min, duration_min, cadence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            s.id,
            s.client_id,
            s.weekday,
            s.start_min,
            s.duration_min,
            s.cadence
        ],
    )?;
    Ok(())
}

pub fn get_slot(conn: &Connection, id: &str) -> Result<Option<Slot>> {
    Ok(conn
        .query_row("SELECT * FROM slot WHERE id = ?1", params![id], slot_from_row)
        .optional()?)
}

pub fn delete_slot(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn.execute("DELETE FROM slot WHERE id = ?1", params![id])? > 0)
}

/// Slot ids beginning with `prefix` (for short-id resolution).
pub fn find_slot_ids_by_prefix(conn: &Connection, prefix: &str) -> Result<Vec<Id>> {
    let cleaned: String = prefix.chars().filter(|c| *c != '%' && *c != '_').collect();
    let pattern = format!("{cleaned}%");
    let mut stmt = conn.prepare("SELECT id FROM slot WHERE id LIKE ?1 ORDER BY id")?;
    let rows = stmt.query_map(params![pattern], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_slots(conn: &Connection) -> Result<Vec<Slot>> {
    let mut stmt = conn.prepare("SELECT * FROM slot ORDER BY weekday, start_min, id")?;
    let rows = stmt.query_map([], slot_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_slots_for(conn: &Connection, client_id: &str) -> Result<Vec<Slot>> {
    let mut stmt =
        conn.prepare("SELECT * FROM slot WHERE client_id = ?1 ORDER BY weekday, start_min")?;
    let rows = stmt.query_map(params![client_id], slot_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn config_set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn config_get(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM config WHERE key = ?1", params![key], |r| {
            r.get::<_, String>(0)
        })
        .optional()?)
}

pub fn config_all(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Cash received per month (`YYYY-MM` → cents), dated at each up-front purchase/renewal.
pub fn revenue_cash_by_month(conn: &Connection) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT strftime('%Y-%m', purchased_at, 'unixepoch') AS ym, SUM(price_cents) AS cash
         FROM package GROUP BY ym ORDER BY ym",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Revenue earned per month (`YYYY-MM` → cents), recognized per completed session as
/// `package price / total sessions`.
pub fn revenue_earned_by_month(conn: &Connection) -> Result<Vec<(String, f64)>> {
    let mut stmt = conn.prepare(
        "SELECT strftime('%Y-%m', s.occurred_at, 'unixepoch') AS ym,
                SUM(CAST(p.price_cents AS REAL) / p.total_sessions) AS earned
         FROM session s JOIN package p ON s.package_id = p.id
         WHERE s.status = 'completed'
         GROUP BY ym ORDER BY ym",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
