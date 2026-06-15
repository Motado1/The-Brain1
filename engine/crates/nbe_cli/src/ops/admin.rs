use super::*;

const CALENDAR_URL_KEY: &str = "calendar_ics_url";

// ---- admin -----------------------------------------------------------------------------

pub fn stats(db: &nbe_data::Db) -> Result<String> {
    let knowledge = repo::list_knowledge(&db.conn)?;
    let topics = knowledge
        .iter()
        .filter(|n| n.template_type.as_deref() == Some("topic"))
        .count();
    Ok(format!(
        "entities: {}\nclients:  {}\nledger:   {}\nnotes:    {}\ntopics:   {}\nedges:    {}",
        repo::count_entities(&db.conn)?,
        repo::list_crm(&db.conn)?.len(),
        repo::list_ledger(&db.conn)?.len(),
        knowledge.len() - topics,
        topics,
        repo::count_edges(&db.conn)?,
    ))
}

/// Recompute every entity's activation from its current facets (via `nbe_sim`) and persist it,
/// so the renderer reads up-to-date values. Each activation's threshold + last_fired_at are
/// preserved. Run after a batch of edits, or before opening the visual engine.
pub fn recompute_activation(db: &mut nbe_data::Db, now: i64) -> Result<String> {
    let ids = repo::all_entity_ids(&db.conn)?;
    let mut hot = 0usize;
    for id in &ids {
        let Some(ewf) = repo::entity_with_facets(&db.conn, id)? else {
            continue;
        };
        let value = activation_value(&ActivationInputs {
            crm: ewf.crm.as_ref(),
            ledger: ewf.ledger.as_ref(),
            knowledge: ewf.knowledge.as_ref(),
            last_fired_at: ewf.activation.as_ref().and_then(|a| a.last_fired_at),
            now,
        });
        let threshold = ewf.activation.as_ref().map(|a| a.threshold).unwrap_or(0.5);
        let last_fired_at = ewf.activation.as_ref().and_then(|a| a.last_fired_at);
        repo::upsert_activation(
            &db.conn,
            &Activation {
                entity_id: id.clone(),
                value,
                threshold,
                last_fired_at,
            },
        )?;
        if value >= threshold {
            hot += 1;
        }
    }
    Ok(format!(
        "recomputed activation for {} entities ({hot} at/above threshold)",
        ids.len()
    ))
}

pub fn seed_demo(db: &mut nbe_data::Db, entities: usize, edges: usize) -> Result<String> {
    nbe_data::seed::seed(
        db,
        &SeedConfig {
            entities,
            edges,
            ..Default::default()
        },
    )?;
    Ok(format!("seeded {entities} entities / {edges} edges"))
}

pub fn export(db: &nbe_data::Db, path: &std::path::Path) -> Result<String> {
    let json = snapshot::export_json_string(db)?;
    std::fs::write(path, json)?;
    Ok(format!("exported snapshot to {}", path.display()))
}

pub fn import(db: &mut nbe_data::Db, path: &std::path::Path) -> Result<String> {
    let json = std::fs::read_to_string(path)?;
    snapshot::import_json_string(db, &json)?;
    Ok(format!("imported snapshot from {}", path.display()))
}

// ---- calendar sync ---------------------------------------------------------------------

pub fn calendar_set_url(db: &nbe_data::Db, url: &str) -> Result<String> {
    repo::config_set(&db.conn, CALENDAR_URL_KEY, url)?;
    Ok("saved Google Calendar ICS URL".into())
}

pub fn calendar_sync(db: &mut nbe_data::Db, file: Option<&std::path::Path>, _now: i64) -> Result<String> {
    let text = match file {
        Some(p) => std::fs::read_to_string(p)?,
        None => {
            let url = repo::config_get(&db.conn, CALENDAR_URL_KEY)?.ok_or_else(|| {
                Error::Msg("no calendar URL set — run calendar-set-url, or pass --file".into())
            })?;
            let src = nbe_calendar::HttpIcsSource { url };
            use nbe_calendar::EventSource;
            src.fetch().map_err(Error::Msg)?
        }
    };

    let events = nbe_calendar::parse_ics(&text);
    let clients: Vec<(String, String)> = repo::list_crm(&db.conn)?
        .into_iter()
        .filter_map(|c| c.contact.map(|name| (c.entity_id, name)))
        .collect();
    let matched = nbe_calendar::match_events(&events, &clients);

    let mut new_count = 0;
    for m in &matched {
        let pkg = repo::active_package(&db.conn, &m.client_id)?;
        let inserted = repo::insert_session(
            &db.conn,
            &Session {
                id: new_id(),
                client_id: m.client_id.clone(),
                package_id: pkg.map(|p| p.id),
                occurred_at: m.occurred_at,
                status: "completed".into(),
                source: "gcal".into(),
                external_id: Some(m.uid.clone()),
                note: Some(m.summary.clone()),
            },
        )?;
        if inserted {
            new_count += 1;
        }
    }
    let unmatched = events.len().saturating_sub(matched.len());
    Ok(format!(
        "calendar: {} events, {} matched, {new_count} new session(s) logged, {unmatched} unmatched",
        events.len(),
        matched.len()
    ))
}
