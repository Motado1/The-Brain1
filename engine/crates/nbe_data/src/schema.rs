//! Schema definition and `user_version`-based migrations.
//!
//! The first read of `PRAGMA user_version` is also what surfaces a wrong/missing SQLCipher
//! key: on an encrypted database opened without the correct key, SQLite reports
//! "file is not a database", which propagates as an error from [`migrate`].

use rusqlite::Connection;

use crate::Result;

/// Current schema version. Bump and add a new block when the schema evolves.
const CURRENT_VERSION: i64 = 3;

/// Apply any pending migrations to bring `conn` up to [`CURRENT_VERSION`].
pub fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version < 1 {
        conn.execute_batch(V1)?;
    }
    if version < 2 {
        conn.execute_batch(V2)?;
    }
    if version < 3 {
        conn.execute_batch(V3)?;
    }
    if version < CURRENT_VERSION {
        conn.execute_batch(&format!("PRAGMA user_version = {CURRENT_VERSION};"))?;
    }

    Ok(())
}

/// v1 — the Entity-Component facet schema. Facets are 1:1 with `entity` and cascade on delete,
/// so a single entity may carry any subset (the client / line-item / knowledge polymorphism).
const V1: &str = r#"
CREATE TABLE entity (
    id          TEXT PRIMARY KEY,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE crm_facet (
    entity_id        TEXT PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
    contact          TEXT,
    lifecycle_stage  TEXT NOT NULL,
    session_schedule TEXT,
    renewal_date     INTEGER
);

CREATE TABLE ledger_facet (
    entity_id           TEXT PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
    amount_cents        INTEGER NOT NULL,
    invoice_status      TEXT NOT NULL,
    is_expense          INTEGER NOT NULL,
    tax_bucket          TEXT,
    pacing_target_cents INTEGER
);

CREATE TABLE knowledge_facet (
    entity_id     TEXT PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
    body_md       TEXT NOT NULL,
    template_type TEXT,
    review_status TEXT NOT NULL
);

CREATE TABLE edge (
    id         TEXT PRIMARY KEY,
    source_id  TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    target_id  TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    edge_type  TEXT NOT NULL,
    weight     REAL NOT NULL DEFAULT 1.0,
    directed   INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_edge_source ON edge(source_id);
CREATE INDEX idx_edge_target ON edge(target_id);

CREATE TABLE activation (
    entity_id     TEXT PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
    value         REAL NOT NULL DEFAULT 0.0,
    threshold     REAL NOT NULL DEFAULT 0.5,
    last_fired_at INTEGER
);

CREATE TABLE layer_assignment (
    entity_id TEXT PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
    layer     TEXT NOT NULL
);
CREATE INDEX idx_layer ON layer_assignment(layer);

CREATE VIEW v_ledger_rollup AS
SELECT
    COALESCE(SUM(amount_cents), 0)                                            AS total_cents,
    COALESCE(SUM(CASE WHEN invoice_status = 'paid' THEN amount_cents ELSE 0 END), 0)  AS paid_cents,
    COALESCE(SUM(CASE WHEN invoice_status <> 'paid' THEN amount_cents ELSE 0 END), 0) AS outstanding_cents,
    COALESCE(SUM(CASE WHEN is_expense = 0 THEN amount_cents ELSE 0 END), 0)   AS income_cents,
    COALESCE(SUM(CASE WHEN is_expense = 1 THEN amount_cents ELSE 0 END), 0)   AS expense_cents
FROM ledger_facet;
"#;

/// v2 — personal-training domain: session packages, logged sessions, recurring weekly slots,
/// and a key/value config store (e.g. the calendar ICS URL).
const V2: &str = r#"
CREATE TABLE package (
    id             TEXT PRIMARY KEY,
    client_id      TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,
    total_sessions INTEGER NOT NULL,
    price_cents    INTEGER NOT NULL,
    purchased_at   INTEGER NOT NULL,
    active         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_package_client ON package(client_id);

CREATE TABLE session (
    id          TEXT PRIMARY KEY,
    client_id   TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    package_id  TEXT REFERENCES package(id) ON DELETE SET NULL,
    occurred_at INTEGER NOT NULL,
    status      TEXT NOT NULL DEFAULT 'completed',
    source      TEXT NOT NULL DEFAULT 'manual',
    external_id TEXT,
    note        TEXT
);
CREATE INDEX idx_session_client ON session(client_id);
CREATE UNIQUE INDEX idx_session_external ON session(external_id) WHERE external_id IS NOT NULL;

CREATE TABLE slot (
    id           TEXT PRIMARY KEY,
    client_id    TEXT NOT NULL REFERENCES entity(id) ON DELETE CASCADE,
    weekday      INTEGER NOT NULL,
    start_min    INTEGER NOT NULL,
    duration_min INTEGER NOT NULL DEFAULT 60,
    cadence      REAL NOT NULL DEFAULT 1.0
);
CREATE INDEX idx_slot_client ON slot(client_id);

CREATE TABLE config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

/// v3 — the client **profile** facet: free-text personal-training fields (fitness goals, dietary
/// needs, injury history) that surface as the "planet" detail nodes orbiting a client. 1:1 with
/// `entity`, cascades on delete like every other facet.
const V3: &str = r#"
CREATE TABLE profile_facet (
    entity_id      TEXT PRIMARY KEY REFERENCES entity(id) ON DELETE CASCADE,
    fitness_goals  TEXT,
    dietary_needs  TEXT,
    injury_history TEXT
);
"#;
