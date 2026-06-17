use super::*;

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

pub(crate) fn label_for(ewf: &nbe_data::EntityWithFacets) -> String {
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
