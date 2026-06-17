//! Command handlers. Each takes `&mut Db` and returns the text to print, so they're directly
//! unit-testable against an in-memory database (no process spawning). `now` is passed in so
//! tests are deterministic.

pub(crate) use nbe_data::{
    new_id, repo, seed::SeedConfig, snapshot, Activation, CrmFacet, Edge, Entity, Error,
    KnowledgeFacet, Layer, LedgerFacet, Package, Result, Session, Slot,
};
pub(crate) use nbe_sim::{activation_value, ActivationInputs};

pub(crate) use crate::datetime::{format_date, parse_date, year_month};
pub(crate) use crate::money::{format_cents, parse_dollars};
pub(crate) use crate::schedule::{fmt_hhmm, parse_hhmm, parse_weekday, weekday_name};

mod admin;
mod clients;
mod pt;
mod reports;
mod research;

pub use admin::*;
pub use clients::*;
pub use pt::*;
pub use reports::*;
pub use research::*;

pub(crate) fn client_name(db: &nbe_data::Db, id: &str) -> Result<String> {
    Ok(repo::get_crm(&db.conn, id)?
        .and_then(|c| c.contact)
        .unwrap_or_else(|| short(id).to_string()))
}

/// Infer the session count from a package kind like "PT10".
pub(crate) fn kind_sessions(kind: &str) -> Result<i64> {
    let digits: String = kind.chars().filter(|c| c.is_ascii_digit()).collect();
    digits
        .parse()
        .map_err(|_| Error::Msg(format!("cannot infer session count from '{kind}' (use e.g. PT10)")))
}

pub(crate) fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

/// Resolve a short id prefix to exactly one entity id, or a helpful error.
pub(crate) fn resolve(db: &nbe_data::Db, prefix: &str) -> Result<String> {
    let mut hits = repo::find_ids_by_prefix(&db.conn, prefix)?;
    match hits.len() {
        1 => Ok(hits.remove(0)),
        0 => Err(Error::Msg(format!("no entity matches id '{prefix}'"))),
        n => Err(Error::Msg(format!("'{prefix}' is ambiguous ({n} matches)"))),
    }
}

/// Resolve a list of candidate ids (from a prefix lookup) to exactly one, or a helpful error.
pub(crate) fn resolve_one(ids: Vec<String>, prefix: &str, kind: &str) -> Result<String> {
    let mut ids = ids;
    match ids.len() {
        1 => Ok(ids.remove(0)),
        0 => Err(Error::Msg(format!("no {kind} matches id '{prefix}'"))),
        n => Err(Error::Msg(format!("'{prefix}' is ambiguous ({n} {kind}s match)"))),
    }
}

pub(crate) fn new_entity(db: &nbe_data::Db, now: i64) -> Result<String> {
    let id = new_id();
    repo::insert_entity(
        &db.conn,
        &Entity {
            id: id.clone(),
            created_at: now,
            updated_at: now,
        },
    )?;
    Ok(id)
}

pub(crate) fn set_activation(db: &nbe_data::Db, id: &str, value: f64) -> Result<()> {
    repo::upsert_activation(
        &db.conn,
        &Activation {
            entity_id: id.into(),
            value,
            threshold: 0.5,
            last_fired_at: None,
        },
    )
}

// ---- renewal / income projection -------------------------------------------------------

/// Projected calendar time at which `remaining` sessions run out, given a weekly `freq`
/// (sessions/week). Anchored at `from`. Returns `None` when there's nothing to project from.
pub(crate) fn project_depletion(from: i64, remaining: i64, freq: f64) -> Option<i64> {
    if freq <= 0.0 {
        return None;
    }
    let weeks = remaining.max(0) as f64 / freq;
    Some(from + (weeks * 7.0 * 86_400.0) as i64)
}

/// Re-derive and store a client's `renewal_date` as the projected depletion of their active
/// package at the current slot cadence. Leaves any existing (e.g. manually set) date untouched
/// when there's no active package or no slots to project from. Returns the new date if set.
pub(crate) fn recompute_renewal(db: &nbe_data::Db, client_id: &str, now: i64) -> Result<Option<i64>> {
    let Some(pkg) = repo::active_package(&db.conn, client_id)? else {
        return Ok(None);
    };
    let freq: f64 = repo::list_slots_for(&db.conn, client_id)?
        .iter()
        .map(|s| s.cadence)
        .sum();
    let used = repo::sessions_completed(&db.conn, &pkg.id)?;
    let remaining = pkg.total_sessions - used;
    let Some(date) = project_depletion(now, remaining, freq) else {
        return Ok(None);
    };
    if let Some(mut crm) = repo::get_crm(&db.conn, client_id)? {
        crm.renewal_date = Some(date);
        repo::upsert_crm(&db.conn, &crm)?;
    }
    Ok(Some(date))
}
