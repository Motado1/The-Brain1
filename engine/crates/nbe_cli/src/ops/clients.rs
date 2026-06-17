use super::*;

// ---- clients ---------------------------------------------------------------------------

pub fn client_add(
    db: &mut nbe_data::Db,
    name: &str,
    stage: &str,
    renewal: Option<&str>,
    schedule: Option<&str>,
    now: i64,
) -> Result<String> {
    let renewal_date = match renewal {
        Some(s) => Some(parse_date(s).map_err(Error::Msg)?),
        None => None,
    };
    let id = new_entity(db, now)?;
    let crm = CrmFacet {
        entity_id: id.clone(),
        contact: Some(name.to_string()),
        lifecycle_stage: stage.to_string(),
        session_schedule: schedule.map(str::to_string),
        renewal_date,
    };
    repo::upsert_crm(&db.conn, &crm)?;
    repo::set_layer(&db.conn, &id, &Layer::Hidden(0))?;
    let act = activation_value(&ActivationInputs {
        crm: Some(&crm),
        now,
        ..Default::default()
    });
    set_activation(db, &id, act)?;
    Ok(format!("added client {} ({name})", short(&id)))
}

pub fn client_list(db: &nbe_data::Db) -> Result<String> {
    let clients = repo::list_crm(&db.conn)?;
    if clients.is_empty() {
        return Ok("no clients yet".into());
    }
    let mut out = format!("{} client(s):\n", clients.len());
    for c in clients {
        let renewal = c
            .renewal_date
            .map(|d| format!("  renews {}", format_date(d)))
            .unwrap_or_default();
        out.push_str(&format!(
            "  {}  {:<24} [{}]{}\n",
            short(&c.entity_id),
            c.contact.as_deref().unwrap_or("(no name)"),
            c.lifecycle_stage,
            renewal,
        ));
    }
    Ok(out.trim_end().to_string())
}

// ---- ledger (invoices & expenses) ------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn invoice_add(
    db: &mut nbe_data::Db,
    amount: &str,
    status: &str,
    client: Option<&str>,
    tax_bucket: Option<&str>,
    now: i64,
) -> Result<String> {
    let amount_cents = parse_dollars(amount).map_err(Error::Msg)?;
    let id = new_entity(db, now)?;
    let ledger = LedgerFacet {
        entity_id: id.clone(),
        amount_cents,
        invoice_status: status.to_string(),
        is_expense: false,
        tax_bucket: Some(tax_bucket.unwrap_or("income").to_string()),
        pacing_target_cents: None,
    };
    repo::upsert_ledger(&db.conn, &ledger)?;
    repo::set_layer(&db.conn, &id, &Layer::Output)?;
    set_activation(db, &id, if status == "overdue" { 0.85 } else { 0.4 })?;

    let mut linked = String::new();
    if let Some(c) = client {
        let cid = resolve(db, c)?;
        repo::insert_edge(
            &db.conn,
            &Edge {
                id: new_id(),
                source_id: cid.clone(),
                target_id: id.clone(),
                edge_type: "flow".into(),
                weight: 1.0,
                directed: true,
            },
        )?;
        linked = format!(" from client {}", short(&cid));
    }
    Ok(format!(
        "added invoice {} {} [{status}]{linked}",
        short(&id),
        format_cents(amount_cents)
    ))
}

pub fn expense_add(
    db: &mut nbe_data::Db,
    amount: &str,
    tax_bucket: &str,
    now: i64,
) -> Result<String> {
    let amount_cents = parse_dollars(amount).map_err(Error::Msg)?;
    let id = new_entity(db, now)?;
    repo::upsert_ledger(
        &db.conn,
        &LedgerFacet {
            entity_id: id.clone(),
            amount_cents,
            invoice_status: "paid".into(),
            is_expense: true,
            tax_bucket: Some(tax_bucket.to_string()),
            pacing_target_cents: None,
        },
    )?;
    repo::set_layer(&db.conn, &id, &Layer::Input)?;
    set_activation(db, &id, 0.2)?;
    Ok(format!(
        "added expense {} {} [{tax_bucket}]",
        short(&id),
        format_cents(amount_cents)
    ))
}

// ---- linking ---------------------------------------------------------------------------

pub fn link(
    db: &mut nbe_data::Db,
    source: &str,
    target: &str,
    edge_type: &str,
    weight: f64,
) -> Result<String> {
    let s = resolve(db, source)?;
    let t = resolve(db, target)?;
    repo::insert_edge(
        &db.conn,
        &Edge {
            id: new_id(),
            source_id: s.clone(),
            target_id: t.clone(),
            edge_type: edge_type.to_string(),
            weight,
            directed: true,
        },
    )?;
    Ok(format!("linked {} -> {} [{edge_type}]", short(&s), short(&t)))
}

/// Remove directed links from `source` to `target` (optionally only one edge type).
pub fn unlink(
    db: &mut nbe_data::Db,
    source: &str,
    target: &str,
    edge_type: Option<&str>,
) -> Result<String> {
    let s = resolve(db, source)?;
    let t = resolve(db, target)?;
    let removed = repo::delete_edges_between(&db.conn, &s, &t, edge_type)?;
    if removed == 0 {
        let kind = edge_type.map(|t| format!(" [{t}]")).unwrap_or_default();
        return Err(Error::Msg(format!(
            "no link {} -> {}{kind}",
            short(&s),
            short(&t)
        )));
    }
    Ok(format!(
        "removed {removed} link(s) {} -> {}",
        short(&s),
        short(&t)
    ))
}

// ---- edits / state transitions ---------------------------------------------------------

/// Update an existing client in place. Only the fields supplied are changed; an empty
/// `--renewal ""` clears the renewal date. Activation is recomputed from the new facet.
pub fn client_update(
    db: &mut nbe_data::Db,
    prefix: &str,
    name: Option<&str>,
    stage: Option<&str>,
    renewal: Option<&str>,
    schedule: Option<&str>,
    now: i64,
) -> Result<String> {
    let id = resolve(db, prefix)?;
    let mut crm = repo::get_crm(&db.conn, &id)?
        .ok_or_else(|| Error::Msg(format!("{} is not a client", short(&id))))?;
    if let Some(n) = name {
        crm.contact = Some(n.to_string());
    }
    if let Some(s) = stage {
        crm.lifecycle_stage = s.to_string();
    }
    if let Some(s) = schedule {
        crm.session_schedule = Some(s.to_string());
    }
    if let Some(r) = renewal {
        crm.renewal_date = if r.is_empty() {
            None
        } else {
            Some(parse_date(r).map_err(Error::Msg)?)
        };
    }
    repo::upsert_crm(&db.conn, &crm)?;
    let act = activation_value(&ActivationInputs {
        crm: Some(&crm),
        now,
        ..Default::default()
    });
    set_activation(db, &id, act)?;
    Ok(format!(
        "updated client {} ({})",
        short(&id),
        crm.contact.as_deref().unwrap_or("(no name)")
    ))
}

/// Split a stored note body (`# title\n\n…body`) back into its title and body parts.
fn split_note(md: &str) -> (String, String) {
    let mut lines = md.lines();
    let title = lines.next().unwrap_or("").trim_start_matches("# ").to_string();
    let body = lines.collect::<Vec<_>>().join("\n");
    let body = body.strip_prefix('\n').unwrap_or(&body).to_string();
    (title, body)
}

/// Update a research note in place — title, body, and/or review status.
pub fn note_update(
    db: &mut nbe_data::Db,
    prefix: &str,
    title: Option<&str>,
    body: Option<&str>,
    status: Option<&str>,
) -> Result<String> {
    let id = resolve(db, prefix)?;
    let mut note = repo::get_knowledge(&db.conn, &id)?
        .ok_or_else(|| Error::Msg(format!("{} is not a note", short(&id))))?;
    if title.is_some() || body.is_some() {
        let (cur_title, cur_body) = split_note(&note.body_md);
        let new_title = title.unwrap_or(&cur_title);
        let new_body = body.unwrap_or(&cur_body);
        note.body_md = format!("# {new_title}\n\n{new_body}");
    }
    if let Some(s) = status {
        note.review_status = s.to_string();
    }
    repo::upsert_knowledge(&db.conn, &note)?;
    Ok(format!("updated note {} [{}]", short(&id), note.review_status))
}

/// Delete an entity and everything that cascades from it (facets, edges, packages, sessions,
/// slots). Irreversible — meant for correcting mistakes.
pub fn delete(db: &mut nbe_data::Db, prefix: &str) -> Result<String> {
    let id = resolve(db, prefix)?;
    let label = repo::entity_with_facets(&db.conn, &id)?
        .as_ref()
        .map(label_for)
        .unwrap_or_else(|| short(&id).to_string());
    if repo::delete_entity(&db.conn, &id)? {
        Ok(format!("deleted {} ({label}) and its links", short(&id)))
    } else {
        Err(Error::Msg(format!("nothing to delete at {}", short(&id))))
    }
}
