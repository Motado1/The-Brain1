//! Command handlers. Each takes `&mut Db` and returns the text to print, so they're directly
//! unit-testable against an in-memory database (no process spawning). `now` is passed in so
//! tests are deterministic.

use nbe_data::{
    new_id, repo, seed::SeedConfig, snapshot, Activation, CrmFacet, Edge, Entity, Error,
    KnowledgeFacet, Layer, LedgerFacet, Package, Result, Session, Slot,
};
use nbe_sim::{activation_value, ActivationInputs};

use crate::datetime::{format_date, parse_date, year_month};
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

/// Resolve a list of candidate ids (from a prefix lookup) to exactly one, or a helpful error.
fn resolve_one(ids: Vec<String>, prefix: &str, kind: &str) -> Result<String> {
    let mut ids = ids;
    match ids.len() {
        1 => Ok(ids.remove(0)),
        0 => Err(Error::Msg(format!("no {kind} matches id '{prefix}'"))),
        n => Err(Error::Msg(format!("'{prefix}' is ambiguous ({n} {kind}s match)"))),
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

// ---- renewal / income projection -------------------------------------------------------

/// Projected calendar time at which `remaining` sessions run out, given a weekly `freq`
/// (sessions/week). Anchored at `from`. Returns `None` when there's nothing to project from.
fn project_depletion(from: i64, remaining: i64, freq: f64) -> Option<i64> {
    if freq <= 0.0 {
        return None;
    }
    let weeks = remaining.max(0) as f64 / freq;
    Some(from + (weeks * 7.0 * 86_400.0) as i64)
}

/// Re-derive and store a client's `renewal_date` as the projected depletion of their active
/// package at the current slot cadence. Leaves any existing (e.g. manually set) date untouched
/// when there's no active package or no slots to project from. Returns the new date if set.
fn recompute_renewal(db: &nbe_data::Db, client_id: &str, now: i64) -> Result<Option<i64>> {
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

/// List research notes. With `tag`, only notes linked to that topic; otherwise all notes
/// (topic neurons themselves are excluded — see `topic_list`).
pub fn note_list(db: &nbe_data::Db, tag: Option<&str>) -> Result<String> {
    if let Some(t) = tag {
        let t = t.trim();
        let topic_id = repo::topic_by_name(&db.conn, t)?
            .ok_or_else(|| Error::Msg(format!("no topic '{t}'")))?;
        let mut rows: Vec<(String, String, String)> = Vec::new();
        for e in repo::backlinks(&db.conn, &topic_id)? {
            if e.edge_type != "topic" {
                continue;
            }
            if let Some(k) = repo::get_knowledge(&db.conn, &e.source_id)? {
                let title = k.body_md.lines().next().unwrap_or("").trim_start_matches("# ");
                rows.push((short(&e.source_id).to_string(), title.to_string(), k.review_status));
            }
        }
        if rows.is_empty() {
            return Ok(format!("no notes tagged '{t}'"));
        }
        rows.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        let mut out = format!("{} note(s) tagged '{t}':\n", rows.len());
        for (id, title, status) in rows {
            out.push_str(&format!("  {id}  {title:<32} [{status}]\n"));
        }
        return Ok(out.trim_end().to_string());
    }

    let notes: Vec<_> = repo::list_knowledge(&db.conn)?
        .into_iter()
        .filter(|n| n.template_type.as_deref() != Some("topic"))
        .collect();
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

// ---- research tags / topics ------------------------------------------------------------

/// Find an existing topic neuron by name, or create one (a knowledge entity flagged
/// `template_type = "topic"` in the Research region) so notes can link to it.
fn ensure_topic(db: &nbe_data::Db, name: &str, now: i64) -> Result<String> {
    if let Some(id) = repo::topic_by_name(&db.conn, name)? {
        return Ok(id);
    }
    let id = new_entity(db, now)?;
    repo::upsert_knowledge(
        &db.conn,
        &KnowledgeFacet {
            entity_id: id.clone(),
            body_md: format!("# {name}"),
            template_type: Some("topic".into()),
            review_status: "topic".into(),
        },
    )?;
    repo::set_layer(&db.conn, &id, &Layer::Hidden(1))?;
    set_activation(db, &id, 0.0)?;
    Ok(id)
}

/// Tag a note with a topic (creating the topic neuron on first use).
pub fn note_tag(db: &mut nbe_data::Db, note_prefix: &str, topic: &str, now: i64) -> Result<String> {
    let topic = topic.trim();
    if topic.is_empty() {
        return Err(Error::Msg("topic name is empty".into()));
    }
    let note_id = resolve(db, note_prefix)?;
    let k = repo::get_knowledge(&db.conn, &note_id)?
        .ok_or_else(|| Error::Msg(format!("{} is not a note", short(&note_id))))?;
    if k.template_type.as_deref() == Some("topic") {
        return Err(Error::Msg("that entity is a topic, not a note".into()));
    }
    let topic_id = ensure_topic(db, topic, now)?;
    let already = repo::outgoing(&db.conn, &note_id)?
        .into_iter()
        .any(|e| e.target_id == topic_id && e.edge_type == "topic");
    if already {
        return Ok(format!("note {} already tagged '{topic}'", short(&note_id)));
    }
    repo::insert_edge(
        &db.conn,
        &Edge {
            id: new_id(),
            source_id: note_id.clone(),
            target_id: topic_id,
            edge_type: "topic".into(),
            weight: 1.0,
            directed: true,
        },
    )?;
    Ok(format!("tagged note {} with '{topic}'", short(&note_id)))
}

/// Remove a topic tag from a note.
pub fn note_untag(db: &mut nbe_data::Db, note_prefix: &str, topic: &str) -> Result<String> {
    let topic = topic.trim();
    let note_id = resolve(db, note_prefix)?;
    let topic_id = repo::topic_by_name(&db.conn, topic)?
        .ok_or_else(|| Error::Msg(format!("no topic '{topic}'")))?;
    let removed = repo::delete_edges_between(&db.conn, &note_id, &topic_id, Some("topic"))?;
    if removed == 0 {
        return Err(Error::Msg(format!(
            "note {} isn't tagged '{topic}'",
            short(&note_id)
        )));
    }
    Ok(format!("untagged note {} from '{topic}'", short(&note_id)))
}

/// Brush-up view: notes (optionally in one topic) ordered least-recently-reviewed first, so the
/// things you haven't revisited surface to the top. "Reviewed" = the note's neuron last fired
/// (`activation.last_fired_at`), which the activation rules already treat as recall recency.
pub fn review(db: &nbe_data::Db, tag: Option<&str>, limit: usize, _now: i64) -> Result<String> {
    let mut ids: Vec<String> = Vec::new();
    if let Some(t) = tag {
        let t = t.trim();
        let topic_id = repo::topic_by_name(&db.conn, t)?
            .ok_or_else(|| Error::Msg(format!("no topic '{t}'")))?;
        for e in repo::backlinks(&db.conn, &topic_id)? {
            if e.edge_type == "topic" {
                ids.push(e.source_id);
            }
        }
    } else {
        for k in repo::list_knowledge(&db.conn)? {
            if k.template_type.as_deref() != Some("topic") {
                ids.push(k.entity_id);
            }
        }
    }

    // (last_reviewed, short id, title, status)
    let mut rows: Vec<(Option<i64>, String, String, String)> = Vec::new();
    for id in ids {
        let Some(k) = repo::get_knowledge(&db.conn, &id)? else {
            continue;
        };
        if k.template_type.as_deref() == Some("topic") {
            continue;
        }
        let last = repo::get_activation(&db.conn, &id)?.and_then(|a| a.last_fired_at);
        let title = k.body_md.lines().next().unwrap_or("").trim_start_matches("# ").to_string();
        rows.push((last, short(&id).to_string(), title, k.review_status));
    }
    if rows.is_empty() {
        return Ok(match tag {
            Some(t) => format!("no notes tagged '{}'", t.trim()),
            None => "no notes to review".into(),
        });
    }
    rows.sort_by(|a, b| {
        a.0.unwrap_or(i64::MIN)
            .cmp(&b.0.unwrap_or(i64::MIN))
            .then(a.2.to_lowercase().cmp(&b.2.to_lowercase()))
    });
    rows.truncate(limit.max(1));

    let scope = tag.map(|t| format!(" in '{}'", t.trim())).unwrap_or_default();
    let mut out = format!(
        "Brush up{scope} — {} note(s), least-recently reviewed first:\n",
        rows.len()
    );
    for (last, id, title, status) in rows {
        let when = last.map(format_date).unwrap_or_else(|| "never".into());
        out.push_str(&format!("  {id}  {title:<32} [{status}]  reviewed {when}\n"));
    }
    out.push_str("  (read one: nbe show <id>; mark done: nbe note-review <id>)");
    Ok(out.trim_end().to_string())
}

/// Mark a note reviewed now — fires its neuron (sets `last_fired_at`), so it lights up in the
/// brain and then cools over the recall window, sinking back down the brush-up list over time.
pub fn note_review(db: &mut nbe_data::Db, note_prefix: &str, now: i64) -> Result<String> {
    let id = resolve(db, note_prefix)?;
    let k = repo::get_knowledge(&db.conn, &id)?
        .ok_or_else(|| Error::Msg(format!("{} is not a note", short(&id))))?;
    if k.template_type.as_deref() == Some("topic") {
        return Err(Error::Msg("that entity is a topic, not a note".into()));
    }
    let threshold = repo::get_activation(&db.conn, &id)?
        .map(|a| a.threshold)
        .unwrap_or(0.5);
    let value = activation_value(&ActivationInputs {
        knowledge: Some(&k),
        last_fired_at: Some(now),
        now,
        ..Default::default()
    });
    repo::upsert_activation(
        &db.conn,
        &Activation {
            entity_id: id.clone(),
            value,
            threshold,
            last_fired_at: Some(now),
        },
    )?;
    let title = k.body_md.lines().next().unwrap_or("").trim_start_matches("# ");
    Ok(format!(
        "reviewed '{title}' — it lights up now and cools over the next 7 days"
    ))
}

/// Parse a lightweight import header: a YAML-ish front-matter block (`---`…`---`) or leading
/// `Title:` / `Tags:` lines before the first blank line. Returns `(title, tags, body)` with the
/// header stripped from the body.
fn parse_import_header(content: &str) -> (Option<String>, Vec<String>, String) {
    let mut title = None;
    let mut tags: Vec<String> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Front-matter block.
    if lines.first().map(|l| l.trim()) == Some("---") {
        if let Some(rel_end) = lines.iter().skip(1).position(|l| l.trim() == "---") {
            let close = rel_end + 1; // index of the closing ---
            for l in &lines[1..close] {
                parse_header_kv(l, &mut title, &mut tags);
            }
            let body = lines[close + 1..].join("\n");
            return (title, tags, body.trim_start_matches('\n').to_string());
        }
    }

    // Leading Title:/Tags: lines.
    let mut consumed = 0;
    for l in &lines {
        let t = l.trim();
        if t.is_empty() {
            break;
        }
        let lower = t.to_ascii_lowercase();
        if lower.starts_with("title:") || lower.starts_with("tags:") {
            parse_header_kv(l, &mut title, &mut tags);
            consumed += 1;
        } else {
            break;
        }
    }
    if consumed > 0 {
        let body = lines[consumed..].join("\n");
        return (title, tags, body.trim_start_matches('\n').to_string());
    }

    (None, tags, content.to_string())
}

fn parse_header_kv(line: &str, title: &mut Option<String>, tags: &mut Vec<String>) {
    let Some((k, v)) = line.split_once(':') else {
        return;
    };
    let val = v.trim().trim_matches(['"', '\'']).trim();
    match k.trim().to_ascii_lowercase().as_str() {
        "title" if !val.is_empty() => *title = Some(val.to_string()),
        "tags" => {
            let val = val.trim_start_matches('[').trim_end_matches(']');
            for part in val.split(',') {
                let p = part.trim().trim_matches(['"', '\'']).trim();
                if !p.is_empty() {
                    tags.push(p.to_string());
                }
            }
        }
        _ => {}
    }
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|l| l.trim().strip_prefix("# ").map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
}

/// Import a markdown file (e.g. a Gemini research doc) as a note neuron, linking it to one or more
/// topic hubs. Topics come from a `Tags:` line/front-matter in the file plus any passed in. Offline
/// — the file is the bridge; The Brain never calls a cloud API.
pub fn note_import(
    db: &mut nbe_data::Db,
    path: &std::path::Path,
    topics: &[String],
    title: Option<&str>,
    status: &str,
    now: i64,
) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    let (parsed_title, parsed_tags, body) = parse_import_header(&content);

    let title_str = title
        .map(str::to_string)
        .or(parsed_title)
        .or_else(|| first_heading(&body))
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Untitled".into());

    // Drop a leading H1 (we re-add an authoritative one) so there's exactly one title line.
    let body_inner = {
        let t = body.trim_start();
        match t.strip_prefix("# ") {
            Some(rest) => rest.split_once('\n').map(|(_, a)| a).unwrap_or("").trim_start_matches('\n'),
            None => t,
        }
    };
    let body_md = format!("# {title_str}\n\n{body_inner}").trim_end().to_string();

    let id = new_entity(db, now)?;
    repo::upsert_knowledge(
        &db.conn,
        &KnowledgeFacet {
            entity_id: id.clone(),
            body_md,
            template_type: None,
            review_status: status.to_string(),
        },
    )?;
    repo::set_layer(&db.conn, &id, &Layer::Hidden(1))?;
    set_activation(db, &id, 0.3)?;

    // Merge in-doc tags with passed topics, dedupe case-insensitively.
    let mut all_topics: Vec<String> = Vec::new();
    for t in parsed_tags.iter().chain(topics.iter()) {
        let t = t.trim();
        if !t.is_empty() && !all_topics.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            all_topics.push(t.to_string());
        }
    }
    for t in &all_topics {
        let topic_id = ensure_topic(db, t, now)?;
        let already = repo::outgoing(&db.conn, &id)?
            .into_iter()
            .any(|e| e.target_id == topic_id && e.edge_type == "topic");
        if !already {
            repo::insert_edge(
                &db.conn,
                &Edge {
                    id: new_id(),
                    source_id: id.clone(),
                    target_id: topic_id,
                    edge_type: "topic".into(),
                    weight: 1.0,
                    directed: true,
                },
            )?;
        }
    }

    let tail = if all_topics.is_empty() {
        " (no tags — add a Tags: line or pass a topic)".to_string()
    } else {
        format!("; tagged [{}]", all_topics.join(", "))
    };
    Ok(format!("imported '{title_str}' as note {}{tail}", short(&id)))
}

/// List every topic with how many notes carry it.
pub fn topic_list(db: &nbe_data::Db) -> Result<String> {
    let topics = repo::list_topics(&db.conn)?;
    if topics.is_empty() {
        return Ok("no topics yet".into());
    }
    let mut out = format!("{} topic(s):\n", topics.len());
    for (id, name) in topics {
        let count = repo::backlinks(&db.conn, &id)?
            .into_iter()
            .filter(|e| e.edge_type == "topic")
            .count();
        out.push_str(&format!("  {name:<24} {count} note(s)\n"));
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
    let renews = recompute_renewal(db, &cid, purchased_at)?
        .map(|d| format!("; renews ~{}", format_date(d)))
        .unwrap_or_else(|| " (add slots to project renewal)".into());
    Ok(format!(
        "added {kind} ({total} sessions, {} up front) for {}{renews}",
        format_cents(price_cents),
        client_name(db, &cid)?
    ))
}

pub fn package_list(db: &nbe_data::Db, client: Option<&str>) -> Result<String> {
    let all = repo::list_packages(&db.conn)?;
    let packages: Vec<_> = match client {
        Some(c) => {
            let cid = resolve(db, c)?;
            all.into_iter().filter(|p| p.client_id == cid).collect()
        }
        None => all,
    };
    if packages.is_empty() {
        return Ok("no packages yet".into());
    }
    let mut out = format!("{} package(s):\n", packages.len());
    for p in packages {
        let used = repo::sessions_completed(&db.conn, &p.id)?;
        let flag = if p.active { "active " } else { "       " };
        out.push_str(&format!(
            "  {}  {flag} {:<18} {} {}/{}  {}\n",
            short(&p.id),
            client_name(db, &p.client_id)?,
            p.kind,
            used,
            p.total_sessions,
            format_cents(p.price_cents),
        ));
    }
    Ok(out.trim_end().to_string())
}

/// Delete a package by short id. Sessions keep their history (their `package_id` is set NULL by
/// the schema). Deleting the *active* package restores the client's previous package as active,
/// so undoing a mistaken renewal puts things back.
pub fn package_delete(db: &mut nbe_data::Db, prefix: &str, now: i64) -> Result<String> {
    let id = resolve_one(repo::find_package_ids_by_prefix(&db.conn, prefix)?, prefix, "package")?;
    let pkg = repo::get_package(&db.conn, &id)?
        .ok_or_else(|| Error::Msg("package vanished".into()))?;
    repo::delete_package(&db.conn, &id)?;
    let mut msg = format!(
        "deleted {} {} for {}",
        short(&id),
        pkg.kind,
        client_name(db, &pkg.client_id)?
    );
    if pkg.active {
        if let Some(prev) = repo::latest_package(&db.conn, &pkg.client_id)? {
            repo::set_package_active(&db.conn, &prev.id, true)?;
            msg.push_str(&format!("; reactivated previous {}", prev.kind));
        }
        recompute_renewal(db, &pkg.client_id, now)?;
    }
    Ok(msg)
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
    recompute_renewal(db, &cid, now)?;

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

#[allow(clippy::too_many_arguments)]
pub fn slot_add(
    db: &mut nbe_data::Db,
    client: &str,
    day: &str,
    time: &str,
    duration_min: i64,
    cadence: f64,
    now: i64,
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
    recompute_renewal(db, &cid, now)?;
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
            "  {}  {} {}  {:<22} {}min  x{}\n",
            short(&s.id),
            weekday_name(s.weekday),
            fmt_hhmm(s.start_min),
            client_name(db, &s.client_id)?,
            s.duration_min,
            s.cadence
        ));
    }
    Ok(out.trim_end().to_string())
}

// ---- session / slot edits --------------------------------------------------------------

/// List a client's sessions (or all sessions) with their short ids for editing.
pub fn session_list(db: &nbe_data::Db, client: Option<&str>) -> Result<String> {
    let sessions = match client {
        Some(c) => {
            let cid = resolve(db, c)?;
            repo::list_sessions_for(&db.conn, &cid)?
        }
        None => repo::list_sessions(&db.conn)?,
    };
    if sessions.is_empty() {
        return Ok("no sessions yet".into());
    }
    let mut out = format!("{} session(s):\n", sessions.len());
    for s in sessions {
        let note = s.note.as_deref().map(|n| format!("  {n}")).unwrap_or_default();
        out.push_str(&format!(
            "  {}  {}  {:<20} [{}]{}\n",
            short(&s.id),
            format_date(s.occurred_at),
            client_name(db, &s.client_id)?,
            s.status,
            note,
        ));
    }
    Ok(out.trim_end().to_string())
}

/// Correct a logged session in place — date, status (completed|no_show|cancelled), and/or note.
pub fn session_update(
    db: &mut nbe_data::Db,
    prefix: &str,
    date: Option<&str>,
    status: Option<&str>,
    note: Option<&str>,
    now: i64,
) -> Result<String> {
    let id = resolve_one(repo::find_session_ids_by_prefix(&db.conn, prefix)?, prefix, "session")?;
    let mut session = repo::get_session(&db.conn, &id)?
        .ok_or_else(|| Error::Msg("session vanished".into()))?;
    if let Some(d) = date {
        session.occurred_at = parse_date(d).map_err(Error::Msg)?;
    }
    if let Some(s) = status {
        session.status = s.to_string();
    }
    if let Some(n) = note {
        session.note = Some(n.to_string());
    }
    repo::replace_session(&db.conn, &session)?;
    recompute_renewal(db, &session.client_id, now)?;
    Ok(format!(
        "updated session {} for {} [{}]",
        short(&id),
        client_name(db, &session.client_id)?,
        session.status
    ))
}

pub fn session_delete(db: &mut nbe_data::Db, prefix: &str, now: i64) -> Result<String> {
    let id = resolve_one(repo::find_session_ids_by_prefix(&db.conn, prefix)?, prefix, "session")?;
    let client_id = repo::get_session(&db.conn, &id)?.map(|s| s.client_id);
    if repo::delete_session(&db.conn, &id)? {
        if let Some(cid) = client_id {
            recompute_renewal(db, &cid, now)?;
        }
        Ok(format!("deleted session {}", short(&id)))
    } else {
        Err(Error::Msg(format!("nothing to delete at {}", short(&id))))
    }
}

/// Edit a recurring slot in place — weekday, time, duration, and/or cadence.
#[allow(clippy::too_many_arguments)]
pub fn slot_update(
    db: &mut nbe_data::Db,
    prefix: &str,
    day: Option<&str>,
    time: Option<&str>,
    duration_min: Option<i64>,
    cadence: Option<f64>,
    now: i64,
) -> Result<String> {
    let id = resolve_one(repo::find_slot_ids_by_prefix(&db.conn, prefix)?, prefix, "slot")?;
    let mut slot = repo::get_slot(&db.conn, &id)?
        .ok_or_else(|| Error::Msg("slot vanished".into()))?;
    if let Some(d) = day {
        slot.weekday = parse_weekday(d).map_err(Error::Msg)?;
    }
    if let Some(t) = time {
        slot.start_min = parse_hhmm(t).map_err(Error::Msg)?;
    }
    if let Some(d) = duration_min {
        slot.duration_min = d;
    }
    if let Some(c) = cadence {
        slot.cadence = c;
    }
    repo::insert_slot(&db.conn, &slot)?;
    recompute_renewal(db, &slot.client_id, now)?;
    Ok(format!(
        "updated slot {} — {} {} {}min x{} for {}",
        short(&id),
        weekday_name(slot.weekday),
        fmt_hhmm(slot.start_min),
        slot.duration_min,
        slot.cadence,
        client_name(db, &slot.client_id)?
    ))
}

pub fn slot_delete(db: &mut nbe_data::Db, prefix: &str, now: i64) -> Result<String> {
    let id = resolve_one(repo::find_slot_ids_by_prefix(&db.conn, prefix)?, prefix, "slot")?;
    let client_id = repo::get_slot(&db.conn, &id)?.map(|s| s.client_id);
    if repo::delete_slot(&db.conn, &id)? {
        if let Some(cid) = client_id {
            recompute_renewal(db, &cid, now)?;
        }
        Ok(format!("deleted slot {}", short(&id)))
    } else {
        Err(Error::Msg(format!("nothing to delete at {}", short(&id))))
    }
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

/// Forward income forecast. Since packages are paid in full up front, each *future* renewal
/// drops its whole price as cash in the month the client's current package is projected to
/// deplete — then again every full package-cycle after that, assuming like-for-like renewal.
/// Buckets the next `months` months by `YYYY-MM`.
pub fn report_forecast(db: &nbe_data::Db, months: i64, now: i64) -> Result<String> {
    use std::collections::BTreeMap;
    let months = months.max(1);
    let (mut y, mut m) = year_month(now);
    let mut order: Vec<String> = Vec::with_capacity(months as usize);
    for _ in 0..months {
        order.push(format!("{y:04}-{m:02}"));
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    let last = order.last().cloned().unwrap_or_default();
    let mut buckets: BTreeMap<String, i64> = order.iter().map(|k| (k.clone(), 0)).collect();

    let mut unprojectable = 0;
    for pkg in repo::active_packages(&db.conn)? {
        if pkg.total_sessions <= 0 {
            continue;
        }
        let freq: f64 = repo::list_slots_for(&db.conn, &pkg.client_id)?
            .iter()
            .map(|s| s.cadence)
            .sum();
        if freq <= 0.0 {
            unprojectable += 1;
            continue;
        }
        let used = repo::sessions_completed(&db.conn, &pkg.id)?;
        let remaining = pkg.total_sessions - used;
        // first future renewal = when the current package depletes; then every full cycle.
        let Some(mut date) = project_depletion(now, remaining, freq) else {
            continue;
        };
        let cycle = project_depletion(0, pkg.total_sessions, freq)
            .unwrap_or(0)
            .max(86_400);
        for _ in 0..600 {
            let (yy, mm) = year_month(date);
            let key = format!("{yy:04}-{mm:02}");
            if key.as_str() > last.as_str() {
                break;
            }
            if let Some(b) = buckets.get_mut(&key) {
                *b += pkg.price_cents;
            }
            date += cycle;
        }
    }

    let mut out = format!("Projected income — cash up front, next {months} month(s):\n");
    let mut total = 0;
    for k in &order {
        let v = buckets[k];
        total += v;
        out.push_str(&format!("  {k}   {}\n", format_cents(v)));
    }
    out.push_str(&format!("  ── total {}", format_cents(total)));
    if unprojectable > 0 {
        out.push_str(&format!(
            "\n  ({unprojectable} active package(s) without slots — add slots to include them)"
        ));
    }
    Ok(out.trim_end().to_string())
}

/// Day-by-day schedule for the next `days` days, derived from the recurring weekly slots, with a
/// ✓ against any client already logged (completed) that day. The trainer's at-a-glance week.
pub fn agenda(db: &nbe_data::Db, days: i64, now: i64) -> Result<String> {
    use std::collections::HashSet;
    let slots = repo::list_slots(&db.conn)?;
    if slots.is_empty() {
        return Ok("no slots scheduled".into());
    }
    // (client, day-since-epoch) pairs that already have a completed session.
    let logged: HashSet<(String, i64)> = repo::list_sessions(&db.conn)?
        .into_iter()
        .filter(|s| s.status == "completed")
        .map(|s| (s.client_id, s.occurred_at.div_euclid(86_400)))
        .collect();

    let today = now.div_euclid(86_400);
    let mut out = format!("Agenda — next {days} day(s):\n");
    let mut any = false;
    for off in 0..days.max(0) {
        let day = today + off;
        let wd = crate::datetime::weekday_from_epoch(day * 86_400);
        let mut day_slots: Vec<&Slot> = slots.iter().filter(|s| s.weekday == wd).collect();
        if day_slots.is_empty() {
            continue;
        }
        any = true;
        day_slots.sort_by_key(|s| s.start_min);
        out.push_str(&format!("  {} {}:\n", weekday_name(wd), format_date(day * 86_400)));
        for s in day_slots {
            let mark = if logged.contains(&(s.client_id.clone(), day)) {
                "  ✓ logged"
            } else {
                ""
            };
            out.push_str(&format!(
                "    {}  {:<20} {}min{mark}\n",
                fmt_hhmm(s.start_min),
                client_name(db, &s.client_id)?,
                s.duration_min,
            ));
        }
    }
    if !any {
        out.push_str("  (nothing scheduled in this window)\n");
    }
    Ok(out.trim_end().to_string())
}

/// Retention metrics. The core signal for an up-front PT model is the **renewal rate**: of the
/// packages a client has fully used up, how many were followed by another purchase. Also reports
/// repeat-client share, average packages per client, and the lifecycle-stage breakdown.
pub fn report_retention(db: &nbe_data::Db) -> Result<String> {
    use std::collections::{BTreeMap, HashMap};

    let crm = repo::list_crm(&db.conn)?;
    let total_clients = crm.len();
    let mut stages: BTreeMap<String, usize> = BTreeMap::new();
    for c in &crm {
        *stages.entry(c.lifecycle_stage.clone()).or_default() += 1;
    }

    // Packages grouped per client.
    let mut by_client: HashMap<String, Vec<Package>> = HashMap::new();
    for p in repo::list_packages(&db.conn)? {
        by_client.entry(p.client_id.clone()).or_default().push(p);
    }
    let clients_with_pkg = by_client.len();
    let repeat_clients = by_client.values().filter(|v| v.len() >= 2).count();
    let total_pkgs: usize = by_client.values().map(Vec::len).sum();

    // Renewal rate: depleted packages that were followed by a later purchase.
    let mut depleted = 0usize;
    let mut renewed = 0usize;
    for pkgs in by_client.values() {
        for p in pkgs {
            if p.total_sessions <= 0 {
                continue;
            }
            let used = repo::sessions_completed(&db.conn, &p.id)?;
            if used >= p.total_sessions {
                depleted += 1;
                if pkgs.iter().any(|q| q.purchased_at > p.purchased_at) {
                    renewed += 1;
                }
            }
        }
    }

    let pct = |n: usize, d: usize| -> String {
        if d == 0 {
            "n/a".into()
        } else {
            format!("{:.0}%", 100.0 * n as f64 / d as f64)
        }
    };
    let avg_pkgs = if clients_with_pkg == 0 {
        0.0
    } else {
        total_pkgs as f64 / clients_with_pkg as f64
    };

    let mut out = String::from("Retention\n");
    out.push_str(&format!("  clients               {total_clients}\n"));
    for (stage, n) in &stages {
        out.push_str(&format!("    {stage:<18}{n}\n"));
    }
    out.push_str(&format!(
        "  renewal rate          {} ({renewed}/{depleted} depleted packages renewed)\n",
        pct(renewed, depleted)
    ));
    out.push_str(&format!(
        "  repeat clients        {} ({repeat_clients}/{clients_with_pkg} bought >1 package)\n",
        pct(repeat_clients, clients_with_pkg)
    ));
    out.push_str(&format!("  avg packages/client   {avg_pkgs:.2}"));
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

/// Low-session nudges: active packages about to run out, most-urgent first, so a client can be
/// re-sold before they lapse. Flags a package when sessions remaining `<= within_sessions` OR the
/// projected weeks-to-depletion `<= within_weeks`. Clients without slots can't be timed, so they
/// flag only on the session count.
pub fn nudges(
    db: &nbe_data::Db,
    within_sessions: i64,
    within_weeks: f64,
    now: i64,
) -> Result<String> {
    // (remaining, depletion-or-max for sorting, formatted line)
    let mut rows: Vec<(i64, i64, String)> = Vec::new();
    for p in repo::active_packages(&db.conn)? {
        if p.total_sessions <= 0 {
            continue;
        }
        let used = repo::sessions_completed(&db.conn, &p.id)?;
        let remaining = (p.total_sessions - used).max(0);
        let freq: f64 = repo::list_slots_for(&db.conn, &p.client_id)?
            .iter()
            .map(|s| s.cadence)
            .sum();
        let weeks_left = (freq > 0.0).then(|| remaining as f64 / freq);
        let flag = remaining <= within_sessions || weeks_left.is_some_and(|w| w <= within_weeks);
        if !flag {
            continue;
        }
        let depletion = project_depletion(now, remaining, freq);
        let eta = match (depletion, weeks_left) {
            (Some(d), Some(w)) => format!("runs out ~{} ({w:.1} wks @ {freq:.1}/wk)", format_date(d)),
            _ => "no slots — add slots for an ETA".to_string(),
        };
        let line = format!(
            "  {:<20} {} {} of {} left — {}; re-sell {}",
            client_name(db, &p.client_id)?,
            p.kind,
            remaining,
            p.total_sessions,
            eta,
            format_cents(p.price_cents),
        );
        rows.push((remaining, depletion.unwrap_or(i64::MAX), line));
    }
    if rows.is_empty() {
        return Ok(format!(
            "no clients within {within_sessions} session(s) or {within_weeks:.0} week(s) of running out"
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut out = format!("Nudges — {} client(s) to re-sell soon:\n", rows.len());
    for (_, _, line) in rows {
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

/// Morning briefing: the three things that matter *today*, composed into one glance — today's
/// sessions (with logged marks), renewals coming due this week, and packages about to run out.
/// Reuses the already-tested `agenda` / `report_renewals` / `nudges` logic with daily defaults.
pub fn today(db: &nbe_data::Db, now: i64) -> Result<String> {
    let midnight = now.div_euclid(86_400) * 86_400;
    let wd = crate::datetime::weekday_from_epoch(midnight);
    let mut out = format!("Today — {} {}\n\n", weekday_name(wd), format_date(midnight));
    out.push_str("▸ ");
    out.push_str(&agenda(db, 1, now)?);
    out.push_str("\n\n▸ ");
    out.push_str(&report_renewals(db, 7, now)?);
    out.push_str("\n\n▸ ");
    out.push_str(&nudges(db, 2, 2.0, now)?);
    Ok(out)
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
