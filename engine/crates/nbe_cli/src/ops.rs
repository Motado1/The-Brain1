//! Command handlers. Each takes `&mut Db` and returns the text to print, so they're directly
//! unit-testable against an in-memory database (no process spawning). `now` is passed in so
//! tests are deterministic.

use nbe_data::{
    new_id, repo, seed::SeedConfig, snapshot, Activation, CrmFacet, Edge, Entity, Error,
    KnowledgeFacet, Layer, LedgerFacet, Result,
};
use nbe_sim::{activation_value, ActivationInputs};

use crate::datetime::{format_date, parse_date};
use crate::money::{format_cents, parse_dollars};

fn short(id: &str) -> &str {
    &id[..id.len().min(8)]
}

/// Resolve a short id prefix to exactly one entity id, or a helpful error.
fn resolve(db: &nbe_data::Db, prefix: &str) -> Result<String> {
    let mut hits = repo::find_ids_by_prefix(&db.conn, prefix)?;
    match hits.len() {
        1 => Ok(hits.remove(0)),
        0 => Err(Error::Msg(format!("no entity matches id '{prefix}'"))),
        n => Err(Error::Msg(format!("'{prefix}' is ambiguous ({n} matches)"))),
    }
}

fn new_entity(db: &nbe_data::Db, now: i64) -> Result<String> {
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

fn set_activation(db: &nbe_data::Db, id: &str, value: f64) -> Result<()> {
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

// ---- knowledge notes -------------------------------------------------------------------

pub fn note_add(
    db: &mut nbe_data::Db,
    title: &str,
    body: &str,
    template: Option<&str>,
    now: i64,
) -> Result<String> {
    let id = new_entity(db, now)?;
    repo::upsert_knowledge(
        &db.conn,
        &KnowledgeFacet {
            entity_id: id.clone(),
            body_md: format!("# {title}\n\n{body}"),
            template_type: template.map(str::to_string),
            review_status: "draft".into(),
        },
    )?;
    repo::set_layer(&db.conn, &id, &Layer::Hidden(1))?;
    set_activation(db, &id, 0.3)?;
    Ok(format!("added note {} ({title})", short(&id)))
}

pub fn note_list(db: &nbe_data::Db) -> Result<String> {
    let notes = repo::list_knowledge(&db.conn)?;
    if notes.is_empty() {
        return Ok("no notes yet".into());
    }
    let mut out = format!("{} note(s):\n", notes.len());
    for n in notes {
        let title = n.body_md.lines().next().unwrap_or("").trim_start_matches("# ");
        out.push_str(&format!(
            "  {}  {:<32} [{}]\n",
            short(&n.entity_id),
            title,
            n.review_status
        ));
    }
    Ok(out.trim_end().to_string())
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

// ---- reports ---------------------------------------------------------------------------

pub fn report_finance(db: &nbe_data::Db) -> Result<String> {
    let r = repo::ledger_rollup(&db.conn)?;
    let mut out = String::from("Finance\n");
    out.push_str(&format!("  income      {}\n", format_cents(r.income_cents)));
    out.push_str(&format!("  expense     {}\n", format_cents(r.expense_cents)));
    out.push_str(&format!(
        "  net         {}\n",
        format_cents(r.income_cents - r.expense_cents)
    ));
    out.push_str(&format!("  paid        {}\n", format_cents(r.paid_cents)));
    out.push_str(&format!(
        "  outstanding {}\n",
        format_cents(r.outstanding_cents)
    ));
    out.push_str("  by bucket:\n");
    for (bucket, total) in repo::ledger_by_bucket(&db.conn)? {
        out.push_str(&format!("    {:<16} {}\n", bucket, format_cents(total)));
    }
    Ok(out.trim_end().to_string())
}

pub fn report_renewals(db: &nbe_data::Db, within_days: i64, now: i64) -> Result<String> {
    let horizon = now + within_days * 86_400;
    let mut due: Vec<(i64, CrmFacet)> = repo::list_crm(&db.conn)?
        .into_iter()
        .filter_map(|c| c.renewal_date.filter(|&d| d <= horizon).map(|d| (d, c)))
        .collect();
    due.sort_by_key(|(d, _)| *d);

    if due.is_empty() {
        return Ok(format!("no renewals within {within_days} days"));
    }
    let mut out = format!("Renewals within {within_days} days:\n");
    for (d, c) in due {
        let overdue = if d < now { "  (OVERDUE)" } else { "" };
        out.push_str(&format!(
            "  {}  {}  {:<24}{}\n",
            format_date(d),
            short(&c.entity_id),
            c.contact.as_deref().unwrap_or("(no name)"),
            overdue
        ));
    }
    Ok(out.trim_end().to_string())
}

/// Live activation ranking — recomputed from facets via `nbe_sim`.
pub fn report_activation(db: &nbe_data::Db, top: usize, now: i64) -> Result<String> {
    let mut ranked: Vec<(f64, String, String)> = Vec::new();
    for id in repo::all_entity_ids(&db.conn)? {
        let Some(ewf) = repo::entity_with_facets(&db.conn, &id)? else {
            continue;
        };
        let value = activation_value(&ActivationInputs {
            crm: ewf.crm.as_ref(),
            ledger: ewf.ledger.as_ref(),
            knowledge: ewf.knowledge.as_ref(),
            last_fired_at: ewf.activation.as_ref().and_then(|a| a.last_fired_at),
            now,
        });
        ranked.push((value, id, label_for(&ewf)));
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    ranked.truncate(top);

    let mut out = format!("Top {} most active:\n", ranked.len());
    for (v, id, label) in ranked {
        out.push_str(&format!("  {:>5.2}  {}  {}\n", v, short(&id), label));
    }
    Ok(out.trim_end().to_string())
}

fn label_for(ewf: &nbe_data::EntityWithFacets) -> String {
    if let Some(c) = &ewf.crm {
        return c.contact.clone().unwrap_or_else(|| "client".into());
    }
    if let Some(k) = &ewf.knowledge {
        return k
            .body_md
            .lines()
            .next()
            .unwrap_or("note")
            .trim_start_matches("# ")
            .to_string();
    }
    if let Some(l) = &ewf.ledger {
        return format!(
            "{} [{}]",
            format_cents(l.amount_cents),
            l.invoice_status
        );
    }
    "entity".into()
}

pub fn show(db: &nbe_data::Db, prefix: &str, now: i64) -> Result<String> {
    let id = resolve(db, prefix)?;
    let ewf = repo::entity_with_facets(&db.conn, &id)?
        .ok_or_else(|| Error::Msg("entity vanished".into()))?;

    let mut out = format!("Entity {}\n", short(&id));
    if let Some(layer) = &ewf.layer {
        out.push_str(&format!("  layer: {}\n", layer.to_db()));
    }
    if let Some(c) = &ewf.crm {
        out.push_str(&format!(
            "  client: {} [{}]{}\n",
            c.contact.as_deref().unwrap_or("(no name)"),
            c.lifecycle_stage,
            c.renewal_date
                .map(|d| format!(" renews {}", format_date(d)))
                .unwrap_or_default()
        ));
    }
    if let Some(l) = &ewf.ledger {
        let kind = if l.is_expense { "expense" } else { "invoice" };
        out.push_str(&format!(
            "  {kind}: {} [{}] bucket={}\n",
            format_cents(l.amount_cents),
            l.invoice_status,
            l.tax_bucket.as_deref().unwrap_or("(none)")
        ));
    }
    if let Some(k) = &ewf.knowledge {
        out.push_str(&format!("  note [{}]:\n", k.review_status));
        for line in k.body_md.lines() {
            out.push_str(&format!("    {line}\n"));
        }
    }
    let value = activation_value(&ActivationInputs {
        crm: ewf.crm.as_ref(),
        ledger: ewf.ledger.as_ref(),
        knowledge: ewf.knowledge.as_ref(),
        last_fired_at: ewf.activation.as_ref().and_then(|a| a.last_fired_at),
        now,
    });
    out.push_str(&format!("  activation: {value:.2}\n"));

    let back = repo::backlinks(&db.conn, &id)?;
    let outgoing = repo::outgoing(&db.conn, &id)?;
    out.push_str(&format!(
        "  links: {} incoming, {} outgoing\n",
        back.len(),
        outgoing.len()
    ));
    for e in back {
        out.push_str(&format!("    <- {} [{}]\n", short(&e.source_id), e.edge_type));
    }
    for e in outgoing {
        out.push_str(&format!("    -> {} [{}]\n", short(&e.target_id), e.edge_type));
    }
    Ok(out.trim_end().to_string())
}

// ---- admin -----------------------------------------------------------------------------

pub fn stats(db: &nbe_data::Db) -> Result<String> {
    Ok(format!(
        "entities: {}\nclients:  {}\nledger:   {}\nnotes:    {}\nedges:    {}",
        repo::count_entities(&db.conn)?,
        repo::list_crm(&db.conn)?.len(),
        repo::list_ledger(&db.conn)?.len(),
        repo::list_knowledge(&db.conn)?.len(),
        repo::count_edges(&db.conn)?,
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
