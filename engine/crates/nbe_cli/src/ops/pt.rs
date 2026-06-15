use super::*;

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
