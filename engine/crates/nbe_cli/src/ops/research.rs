use super::*;

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
