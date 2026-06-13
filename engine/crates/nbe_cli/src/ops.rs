//! Command handlers. Each takes `&mut Db` and returns the text to print, so they're directly
//! unit-testable against an in-memory database (no process spawning). `now` is passed in so
//! tests are deterministic.

use nbe_data::{
    new_id, repo, seed::SeedConfig, snapshot, Activation, CrmFacet, Edge, Entity, Error,
    KnowledgeFacet, Layer, LedgerFacet, Package, Result, Session, Slot,
};
use nbe_sim::{activation_value, ActivationInputs};

use crate::datetime::{format_date, parse_date};
use crate::money::{format_cents, parse_dollars};
use crate::schedule::{fmt_hhmm, parse_hhmm, parse_weekday, weekday_name};

const CALENDAR_URL_KEY: &str = "calendar_ics_url";

fn client_name(db: &nbe_data::Db, id: &str) -> Result<String> {
    Ok(repo::get_crm(&db.conn, id)?
        .and_then(|c| c.contact)
        .unwrap_or_else(|| short(id).to_string()))
}

/// Infer the session count from a package kind like "PT10".
fn kind_sessions(kind: &str) -> Result<i64> {
    let digits: String = kind.chars().filter(|c| c.is_ascii_digit()).collect();
    digits
        .parse()
        .map_err(|_| Error::Msg(format!("cannot infer session count from '{kind}' (use e.g. PT10)")))
}

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

    if let Some(p) = repo::active_package(&db.conn, &id)? {
        let used = repo::sessions_completed(&db.conn, &p.id)?;
        out.push_str(&format!(
            "  package: {} {}/{} ({} paid up front)\n",
            p.kind,
            used,
            p.total_sessions,
            format_cents(p.price_cents)
        ));
    }
    let slots = repo::list_slots_for(&db.conn, &id)?;
    if !slots.is_empty() {
        let freq: f64 = slots.iter().map(|s| s.cadence).sum();
        out.push_str(&format!("  schedule: {freq:.1}/wk\n"));
        for s in &slots {
            out.push_str(&format!(
                "    {} {} {}min\n",
                weekday_name(s.weekday),
                fmt_hhmm(s.start_min),
                s.duration_min
            ));
        }
    }

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

// ---- PT: packages, sessions, slots -----------------------------------------------------

pub fn package_add(
    db: &mut nbe_data::Db,
    client: &str,
    kind: &str,
    price: &str,
    date: Option<&str>,
    now: i64,
) -> Result<String> {
    let cid = resolve(db, client)?;
    let total = kind_sessions(kind)?;
    let price_cents = parse_dollars(price).map_err(Error::Msg)?;
    let purchased_at = match date {
        Some(s) => parse_date(s).map_err(Error::Msg)?,
        None => now,
    };
    repo::deactivate_packages(&db.conn, &cid)?;
    repo::insert_package(
        &db.conn,
        &Package {
            id: new_id(),
            client_id: cid.clone(),
            kind: kind.to_string(),
            total_sessions: total,
            price_cents,
            purchased_at,
            active: true,
        },
    )?;
    Ok(format!(
        "added {kind} ({total} sessions, {} up front) for {}",
        format_cents(price_cents),
        client_name(db, &cid)?
    ))
}

pub fn session_log(
    db: &mut nbe_data::Db,
    client: &str,
    date: Option<&str>,
    status: &str,
    note: Option<&str>,
    now: i64,
) -> Result<String> {
    let cid = resolve(db, client)?;
    let occurred_at = match date {
        Some(s) => parse_date(s).map_err(Error::Msg)?,
        None => now,
    };
    let pkg = repo::active_package(&db.conn, &cid)?;
    repo::insert_session(
        &db.conn,
        &Session {
            id: new_id(),
            client_id: cid.clone(),
            package_id: pkg.as_ref().map(|p| p.id.clone()),
            occurred_at,
            status: status.to_string(),
            source: "manual".into(),
            external_id: None,
            note: note.map(str::to_string),
        },
    )?;

    let name = client_name(db, &cid)?;
    match &pkg {
        Some(p) => {
            let used = repo::sessions_completed(&db.conn, &p.id)?;
            let warn = if used >= p.total_sessions {
                "  ← package complete, time to renew"
            } else {
                ""
            };
            Ok(format!("logged session for {name} {used}/{}{warn}", p.total_sessions))
        }
        None => Ok(format!(
            "logged session for {name} (no active package — add one with package-add)"
        )),
    }
}

pub fn slot_add(
    db: &mut nbe_data::Db,
    client: &str,
    day: &str,
    time: &str,
    duration_min: i64,
    cadence: f64,
) -> Result<String> {
    let cid = resolve(db, client)?;
    let weekday = parse_weekday(day).map_err(Error::Msg)?;
    let start_min = parse_hhmm(time).map_err(Error::Msg)?;
    repo::insert_slot(
        &db.conn,
        &Slot {
            id: new_id(),
            client_id: cid.clone(),
            weekday,
            start_min,
            duration_min,
            cadence,
        },
    )?;
    Ok(format!(
        "added slot {} {} {}min x{cadence} for {}",
        weekday_name(weekday),
        fmt_hhmm(start_min),
        duration_min,
        client_name(db, &cid)?
    ))
}

pub fn slot_list(db: &nbe_data::Db) -> Result<String> {
    let slots = repo::list_slots(&db.conn)?;
    if slots.is_empty() {
        return Ok("no slots yet".into());
    }
    let mut out = format!("{} slot(s):\n", slots.len());
    for s in slots {
        out.push_str(&format!(
            "  {} {}  {:<22} {}min  x{}\n",
            weekday_name(s.weekday),
            fmt_hhmm(s.start_min),
            client_name(db, &s.client_id)?,
            s.duration_min,
            s.cadence
        ));
    }
    Ok(out.trim_end().to_string())
}

// ---- PT reports ------------------------------------------------------------------------

pub fn report_revenue(db: &nbe_data::Db) -> Result<String> {
    let cash = repo::revenue_cash_by_month(&db.conn)?;
    let earned = repo::revenue_earned_by_month(&db.conn)?;

    let mut out = String::from("Revenue — cash received (paid up front):\n");
    if cash.is_empty() {
        out.push_str("  (none)\n");
    }
    for (ym, c) in &cash {
        out.push_str(&format!("  {ym}   {}\n", format_cents(*c)));
    }
    out.push_str("Revenue — earned (per session delivered):\n");
    if earned.is_empty() {
        out.push_str("  (none)\n");
    }
    for (ym, e) in &earned {
        out.push_str(&format!("  {ym}   {}\n", format_cents(e.round() as i64)));
    }
    Ok(out.trim_end().to_string())
}

pub fn report_hours(db: &nbe_data::Db) -> Result<String> {
    let mut by_day = [0.0_f64; 7];
    for s in repo::list_slots(&db.conn)? {
        if (0..7).contains(&s.weekday) {
            by_day[s.weekday as usize] += (s.duration_min as f64 / 60.0) * s.cadence;
        }
    }
    let total: f64 = by_day.iter().sum();
    let mut out = format!("Weekly work hours: {total:.1}\n");
    for (d, h) in by_day.iter().enumerate() {
        if *h > 0.0 {
            out.push_str(&format!("  {}  {:.1}h\n", weekday_name(d as i64), h));
        }
    }
    Ok(out.trim_end().to_string())
}

pub fn report_sessions(db: &nbe_data::Db, now: i64) -> Result<String> {
    let pkgs = repo::active_packages(&db.conn)?;
    if pkgs.is_empty() {
        return Ok("no active packages".into());
    }
    let mut out = String::from("Active packages:\n");
    for p in pkgs {
        let used = repo::sessions_completed(&db.conn, &p.id)?;
        let remaining = (p.total_sessions - used).max(0);
        let freq: f64 = repo::list_slots_for(&db.conn, &p.client_id)?
            .iter()
            .map(|s| s.cadence)
            .sum();
        let eta = if freq > 0.0 {
            let weeks = remaining as f64 / freq;
            let date = now + (weeks * 7.0 * 86_400.0) as i64;
            format!("renew ~{} ({weeks:.1} wks @ {freq:.1}/wk)", format_date(date))
        } else {
            "add slots for renewal ETA".to_string()
        };
        out.push_str(&format!(
            "  {:<20} {} {}/{} — {} left, {}\n",
            client_name(db, &p.client_id)?,
            p.kind,
            used,
            p.total_sessions,
            remaining,
            eta
        ));
    }
    Ok(out.trim_end().to_string())
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
