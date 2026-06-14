//! End-to-end handler tests — exercise the hub workflows against in-memory databases.

use nbe_cli::datetime::parse_date;
use nbe_cli::ops;
use nbe_data::{repo, Db};

const NOW: i64 = 1_700_000_000;

#[test]
fn finance_workflow_rolls_up() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "Acme Co", "active", None, None, NOW).unwrap();
    let cid = repo::list_crm(&db.conn).unwrap()[0].entity_id.clone();

    ops::invoice_add(&mut db, "1000", "sent", Some(&cid[..8]), Some("income"), NOW).unwrap();
    ops::expense_add(&mut db, "250", "software", NOW).unwrap();

    let fin = ops::report_finance(&db).unwrap();
    assert!(fin.contains("income      $1000.00"), "{fin}");
    assert!(fin.contains("expense     $250.00"), "{fin}");
    assert!(fin.contains("net         $750.00"), "{fin}");
    assert!(fin.contains("software"), "bucket breakdown: {fin}");

    // the invoice is linked back to the client
    let invoice = repo::list_ledger(&db.conn)
        .unwrap()
        .into_iter()
        .find(|l| !l.is_expense)
        .unwrap();
    let back = repo::backlinks(&db.conn, &invoice.entity_id).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].source_id, cid);
}

#[test]
fn renewals_report_finds_upcoming() {
    let mut db = Db::open_in_memory().unwrap();
    let now = parse_date("2026-06-13").unwrap();
    ops::client_add(&mut db, "Soon Ltd", "active", Some("2026-06-20"), None, now).unwrap();
    ops::client_add(&mut db, "Later Ltd", "active", Some("2027-01-01"), None, now).unwrap();

    let r = ops::report_renewals(&db, 30, now).unwrap();
    assert!(r.contains("Soon Ltd"), "{r}");
    assert!(!r.contains("Later Ltd"), "out of window: {r}");
}

#[test]
fn link_then_show_lists_connections() {
    let mut db = Db::open_in_memory().unwrap();
    ops::note_add(&mut db, "Topic A", "body a", None, NOW).unwrap();
    ops::note_add(&mut db, "Topic B", "body b", None, NOW).unwrap();
    let ids: Vec<String> = repo::list_knowledge(&db.conn)
        .unwrap()
        .into_iter()
        .map(|k| k.entity_id)
        .collect();

    ops::link(&mut db, &ids[0][..8], &ids[1][..8], "reference", 1.0).unwrap();

    let target = ops::show(&db, &ids[1][..8], NOW).unwrap();
    assert!(target.contains("1 incoming"), "{target}");
    let source = ops::show(&db, &ids[0][..8], NOW).unwrap();
    assert!(source.contains("1 outgoing"), "{source}");
}

#[test]
fn id_resolution_errors_are_helpful() {
    let db = Db::open_in_memory().unwrap();
    let err = ops::show(&db, "deadbeef", NOW).unwrap_err();
    assert!(err.to_string().contains("no entity matches"), "{err}");
}

#[test]
fn activation_report_ranks_urgent_items() {
    let mut db = Db::open_in_memory().unwrap();
    ops::invoice_add(&mut db, "500", "overdue", None, None, NOW).unwrap();
    ops::note_add(&mut db, "calm note", "x", None, NOW).unwrap();

    let r = ops::report_activation(&db, 5, NOW).unwrap();
    // the overdue invoice (0.85) should rank at the top
    let first_value_line = r.lines().nth(1).unwrap_or("");
    assert!(first_value_line.contains("0.85"), "top should be overdue: {r}");
}

#[test]
fn snapshot_export_import_via_files() {
    let dir = tempfile::tempdir().unwrap();
    let snap = dir.path().join("snap.json");

    let mut src = Db::open_in_memory().unwrap();
    ops::client_add(&mut src, "Acme", "active", None, None, NOW).unwrap();
    ops::export(&src, &snap).unwrap();

    let mut dst = Db::open_in_memory().unwrap();
    ops::import(&mut dst, &snap).unwrap();
    assert_eq!(repo::list_crm(&dst.conn).unwrap().len(), 1);
}

fn client_id(db: &Db) -> String {
    repo::list_crm(&db.conn).unwrap()[0].entity_id.clone()
}

#[test]
fn pt_package_lifecycle_and_renewal() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "James Bywater", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);

    ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    let mut last = String::new();
    for i in 0..10 {
        last = ops::session_log(&mut db, &cid[..8], None, "completed", None, NOW + i * 86_400).unwrap();
    }
    assert!(last.contains("10/10"), "{last}");
    assert!(last.contains("renew"), "should flag renewal: {last}");

    // renew with a bigger package
    ops::package_add(&mut db, &cid[..8], "PT20", "1900", None, NOW).unwrap();
    let active = repo::active_package(&db.conn, &cid).unwrap().unwrap();
    assert_eq!(active.kind, "PT20");
    assert_eq!(active.total_sessions, 20);
}

#[test]
fn revenue_report_shows_cash_and_earned() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "Acme", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", Some("2026-06-01"), NOW).unwrap();
    ops::session_log(&mut db, &cid[..8], Some("2026-06-05"), "completed", None, NOW).unwrap();

    let r = ops::report_revenue(&db).unwrap();
    assert!(r.contains("cash received"), "{r}");
    assert!(r.contains("$1000.00"), "full price up front: {r}");
    assert!(r.contains("earned"), "{r}");
    assert!(r.contains("$100.00"), "one session of PT10 = $100 earned: {r}");
}

#[test]
fn slots_drive_work_hours() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "Acme", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::slot_add(&mut db, &cid[..8], "Tue", "09:00", 60, 1.0, NOW).unwrap();
    ops::slot_add(&mut db, &cid[..8], "Thu", "09:00", 30, 1.0, NOW).unwrap();

    let h = ops::report_hours(&db).unwrap();
    assert!(h.contains("Weekly work hours: 1.5"), "{h}");
    assert!(h.contains("Tue  1.0h"), "{h}");
    assert!(h.contains("Thu  0.5h"), "{h}");
}

#[test]
fn client_update_changes_only_supplied_fields() {
    let mut db = Db::open_in_memory().unwrap();
    let now = parse_date("2026-06-14").unwrap();
    ops::client_add(&mut db, "Old Name", "lead", Some("2026-07-01"), None, now).unwrap();
    let cid = client_id(&db);

    // change just the stage; name + renewal must survive.
    ops::client_update(&mut db, &cid[..8], None, Some("active"), None, None, now).unwrap();
    let c = repo::get_crm(&db.conn, &cid).unwrap().unwrap();
    assert_eq!(c.contact.as_deref(), Some("Old Name"));
    assert_eq!(c.lifecycle_stage, "active");
    assert!(c.renewal_date.is_some(), "renewal should be untouched");

    // rename and clear the renewal date.
    ops::client_update(&mut db, &cid[..8], Some("New Name"), None, Some(""), None, now).unwrap();
    let c = repo::get_crm(&db.conn, &cid).unwrap().unwrap();
    assert_eq!(c.contact.as_deref(), Some("New Name"));
    assert_eq!(c.renewal_date, None, "empty --renewal clears it");
}

#[test]
fn note_update_edits_title_body_and_status() {
    let mut db = Db::open_in_memory().unwrap();
    ops::note_add(&mut db, "Draft Title", "first body", None, NOW).unwrap();
    let id = repo::list_knowledge(&db.conn).unwrap()[0].entity_id.clone();

    ops::note_update(&mut db, &id[..8], Some("Final Title"), None, Some("reviewed")).unwrap();
    let k = repo::get_knowledge(&db.conn, &id).unwrap().unwrap();
    assert!(k.body_md.starts_with("# Final Title"), "{}", k.body_md);
    assert!(k.body_md.contains("first body"), "body preserved: {}", k.body_md);
    assert_eq!(k.review_status, "reviewed");
}

#[test]
fn unlink_removes_an_edge() {
    let mut db = Db::open_in_memory().unwrap();
    ops::note_add(&mut db, "A", "a", None, NOW).unwrap();
    ops::note_add(&mut db, "B", "b", None, NOW).unwrap();
    let ids: Vec<String> = repo::list_knowledge(&db.conn)
        .unwrap()
        .into_iter()
        .map(|k| k.entity_id)
        .collect();
    ops::link(&mut db, &ids[0][..8], &ids[1][..8], "reference", 1.0).unwrap();

    ops::unlink(&mut db, &ids[0][..8], &ids[1][..8], None).unwrap();
    assert!(repo::outgoing(&db.conn, &ids[0]).unwrap().is_empty());
    // unlinking again is an error (nothing matches).
    assert!(ops::unlink(&mut db, &ids[0][..8], &ids[1][..8], None).is_err());
}

#[test]
fn delete_cascades_facets_and_edges() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "Doomed", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::note_add(&mut db, "Linked note", "x", None, NOW).unwrap();
    let nid = repo::list_knowledge(&db.conn).unwrap()[0].entity_id.clone();
    ops::link(&mut db, &cid[..8], &nid[..8], "reference", 1.0).unwrap();
    // give the client PT history so cascade has something to clear.
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    ops::session_log(&mut db, &cid[..8], None, "completed", None, NOW).unwrap();

    ops::delete(&mut db, &cid[..8]).unwrap();

    assert!(repo::get_crm(&db.conn, &cid).unwrap().is_none(), "facet gone");
    assert!(repo::backlinks(&db.conn, &nid).unwrap().is_empty(), "edge cascaded");
    assert!(repo::list_packages(&db.conn).unwrap().is_empty(), "package cascaded");
    assert!(repo::list_sessions(&db.conn).unwrap().is_empty(), "session cascaded");
    // the note itself survives.
    assert!(repo::get_knowledge(&db.conn, &nid).unwrap().is_some());
}

#[test]
fn session_can_be_corrected_and_deleted() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "James", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    ops::session_log(&mut db, &cid[..8], None, "completed", None, NOW).unwrap();

    let sid = repo::list_sessions(&db.conn).unwrap()[0].id.clone();
    // a completed session counts against the package...
    let pkg = repo::active_package(&db.conn, &cid).unwrap().unwrap();
    assert_eq!(repo::sessions_completed(&db.conn, &pkg.id).unwrap(), 1);

    // ...cancel it and it no longer counts.
    ops::session_update(&mut db, &sid[..8], None, Some("cancelled"), Some("client ill"), NOW).unwrap();
    assert_eq!(repo::sessions_completed(&db.conn, &pkg.id).unwrap(), 0);
    let s = repo::get_session(&db.conn, &sid).unwrap().unwrap();
    assert_eq!(s.status, "cancelled");
    assert_eq!(s.note.as_deref(), Some("client ill"));

    // delete removes it entirely.
    ops::session_delete(&mut db, &sid[..8], NOW).unwrap();
    assert!(repo::list_sessions(&db.conn).unwrap().is_empty());
}

#[test]
fn slot_can_be_edited_and_removed() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "Acme", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::slot_add(&mut db, &cid[..8], "Tue", "09:00", 60, 1.0, NOW).unwrap();
    let sid = repo::list_slots(&db.conn).unwrap()[0].id.clone();

    // move it to Wednesday and halve the cadence; duration untouched.
    ops::slot_update(&mut db, &sid[..8], Some("Wed"), None, None, Some(0.5), NOW).unwrap();
    let s = repo::get_slot(&db.conn, &sid).unwrap().unwrap();
    assert_eq!(s.weekday, 2, "Wed = 2");
    assert_eq!(s.cadence, 0.5);
    assert_eq!(s.duration_min, 60, "duration preserved");

    ops::slot_delete(&mut db, &sid[..8], NOW).unwrap();
    assert!(repo::list_slots(&db.conn).unwrap().is_empty());
}

#[test]
fn agenda_lists_days_and_marks_logged() {
    let mut db = Db::open_in_memory().unwrap();
    let monday = parse_date("2026-06-15").unwrap(); // a Monday
    ops::client_add(&mut db, "Acme", "active", None, None, monday).unwrap();
    let cid = client_id(&db);
    ops::slot_add(&mut db, &cid[..8], "Mon", "09:00", 60, 1.0, monday).unwrap();

    // before logging: client appears, not yet ✓.
    let before = ops::agenda(&db, 1, monday).unwrap();
    assert!(before.contains("Mon 2026-06-15"), "{before}");
    assert!(before.contains("Acme"), "{before}");
    assert!(!before.contains("✓"), "not logged yet: {before}");

    // after logging a session that day: ✓.
    ops::session_log(&mut db, &cid[..8], Some("2026-06-15"), "completed", None, monday).unwrap();
    let after = ops::agenda(&db, 1, monday).unwrap();
    assert!(after.contains("✓ logged"), "{after}");

    // a window with no matching slots says so (Tue..Sun has none).
    let tuesday = parse_date("2026-06-16").unwrap();
    let empty = ops::agenda(&db, 1, tuesday).unwrap();
    assert!(empty.contains("nothing scheduled"), "{empty}");
}

#[test]
fn deleting_active_package_restores_previous() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "James", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    // original package, then a (mistaken) renewal that supersedes it.
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    ops::package_add(&mut db, &cid[..8], "PT20", "1900", None, NOW + 86_400).unwrap();
    assert_eq!(repo::active_package(&db.conn, &cid).unwrap().unwrap().kind, "PT20");

    // undo the renewal: delete the active PT20 -> PT10 becomes active again.
    let pt20 = repo::list_packages(&db.conn)
        .unwrap()
        .into_iter()
        .find(|p| p.kind == "PT20")
        .unwrap();
    let msg = ops::package_delete(&mut db, &pt20.id[..8], NOW).unwrap();
    assert!(msg.contains("reactivated previous PT10"), "{msg}");
    assert_eq!(repo::active_package(&db.conn, &cid).unwrap().unwrap().kind, "PT10");
    assert_eq!(repo::list_packages(&db.conn).unwrap().len(), 1);
}

#[test]
fn package_add_projects_renewal_from_cadence() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "James", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::slot_add(&mut db, &cid[..8], "Mon", "09:00", 60, 1.0, NOW).unwrap();

    // PT10 at 1/wk depletes in 10 weeks -> renewal_date projected to NOW + 70 days.
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    let week = 7 * 86_400;
    let crm = repo::get_crm(&db.conn, &cid).unwrap().unwrap();
    assert_eq!(crm.renewal_date, Some(NOW + 10 * week), "projected 10 weeks out");

    // logging 4 sessions leaves 6 -> renewal moves to NOW + 6 weeks.
    for _ in 0..4 {
        ops::session_log(&mut db, &cid[..8], None, "completed", None, NOW).unwrap();
    }
    let crm = repo::get_crm(&db.conn, &cid).unwrap().unwrap();
    assert_eq!(crm.renewal_date, Some(NOW + 6 * week), "renewal advances as sessions burn");
}

#[test]
fn package_add_without_slots_cannot_project() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "Solo", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    let msg = ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    assert!(msg.contains("add slots"), "{msg}");
    assert_eq!(repo::get_crm(&db.conn, &cid).unwrap().unwrap().renewal_date, None);
}

#[test]
fn forecast_projects_up_front_income_by_month() {
    let mut db = Db::open_in_memory().unwrap();
    let jan = parse_date("2026-01-01").unwrap();
    ops::client_add(&mut db, "Acme", "active", None, None, jan).unwrap();
    let cid = client_id(&db);
    ops::slot_add(&mut db, &cid[..8], "Mon", "09:00", 60, 1.0, jan).unwrap();
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", Some("2026-01-01"), jan).unwrap();

    // 1/wk + PT10 => renew every 10 weeks (70d): 2026-03-12 and 2026-05-21 fall in a 6mo window.
    let f = ops::report_forecast(&db, 6, jan).unwrap();
    assert!(f.contains("2026-03   $1000.00"), "{f}");
    assert!(f.contains("2026-05   $1000.00"), "{f}");
    assert!(f.contains("total $2000.00"), "{f}");
}

#[test]
fn forecast_flags_clients_without_slots() {
    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "NoSlots", "active", None, None, NOW).unwrap();
    let cid = client_id(&db);
    ops::package_add(&mut db, &cid[..8], "PT10", "1000", None, NOW).unwrap();
    let f = ops::report_forecast(&db, 6, NOW).unwrap();
    assert!(f.contains("without slots"), "{f}");
    assert!(f.contains("total $0.00"), "nothing projectable: {f}");
}

#[test]
fn retention_reports_renewal_and_repeat_rates() {
    let mut db = Db::open_in_memory().unwrap();
    let by_name = |db: &Db, name: &str| -> String {
        repo::list_crm(&db.conn)
            .unwrap()
            .into_iter()
            .find(|c| c.contact.as_deref() == Some(name))
            .unwrap()
            .entity_id
    };

    // A: depletes a PT10 then renews -> depleted + renewed.
    ops::client_add(&mut db, "A", "active", None, None, NOW).unwrap();
    let a = by_name(&db, "A");
    ops::package_add(&mut db, &a[..8], "PT10", "1000", Some("2026-01-01"), NOW).unwrap();
    for _ in 0..10 {
        ops::session_log(&mut db, &a[..8], None, "completed", None, NOW).unwrap();
    }
    ops::package_add(&mut db, &a[..8], "PT10", "1000", Some("2026-04-01"), NOW).unwrap();

    // B: depletes but never renews -> depleted, not renewed.
    ops::client_add(&mut db, "B", "churned", None, None, NOW).unwrap();
    let b = by_name(&db, "B");
    ops::package_add(&mut db, &b[..8], "PT10", "1000", None, NOW).unwrap();
    for _ in 0..10 {
        ops::session_log(&mut db, &b[..8], None, "completed", None, NOW).unwrap();
    }

    // C: a fresh package, not yet depleted -> excluded from the renewal rate.
    ops::client_add(&mut db, "C", "active", None, None, NOW).unwrap();
    let c = by_name(&db, "C");
    ops::package_add(&mut db, &c[..8], "PT10", "1000", None, NOW).unwrap();

    let r = ops::report_retention(&db).unwrap();
    assert!(r.contains("clients               3"), "{r}");
    assert!(r.contains("50% (1/2 depleted packages renewed)"), "{r}");
    assert!(r.contains("33% (1/3 bought >1 package)"), "{r}");
    assert!(r.contains("avg packages/client   1.33"), "{r}");
}

#[test]
fn recompute_activation_refreshes_from_facets() {
    let mut db = Db::open_in_memory().unwrap();
    ops::note_add(&mut db, "stale", "x", None, NOW).unwrap();
    let nid = repo::list_knowledge(&db.conn).unwrap()[0].entity_id.clone();
    // note_add seeds a static 0.3...
    assert!((repo::get_activation(&db.conn, &nid).unwrap().unwrap().value - 0.3).abs() < 1e-9);

    // ...but a note with no recall recency is actually cold; recompute corrects it.
    let msg = ops::recompute_activation(&mut db, NOW).unwrap();
    assert!(msg.contains("recomputed activation"), "{msg}");
    assert!(
        repo::get_activation(&db.conn, &nid).unwrap().unwrap().value.abs() < 1e-9,
        "note cooled to 0 after recompute"
    );
}

#[test]
fn notes_tag_into_topics_and_filter() {
    let mut db = Db::open_in_memory().unwrap();
    ops::note_add(&mut db, "Protein timing", "eat protein", None, NOW).unwrap();
    ops::note_add(&mut db, "Squat depth", "below parallel", None, NOW).unwrap();
    let by_title = |db: &Db, title: &str| -> String {
        repo::list_knowledge(&db.conn)
            .unwrap()
            .into_iter()
            .find(|k| k.body_md.starts_with(&format!("# {title}")))
            .unwrap()
            .entity_id
    };
    let protein = by_title(&db, "Protein timing");
    let squat = by_title(&db, "Squat depth");

    ops::note_tag(&mut db, &protein[..8], "Nutrition", NOW).unwrap();
    ops::note_tag(&mut db, &squat[..8], "Technique", NOW).unwrap();
    // re-tagging is a no-op.
    let again = ops::note_tag(&mut db, &protein[..8], "Nutrition", NOW).unwrap();
    assert!(again.contains("already tagged"), "{again}");

    // topics list with counts.
    let topics = ops::topic_list(&db).unwrap();
    assert!(topics.contains("Nutrition") && topics.contains("Technique"), "{topics}");
    assert!(topics.contains("1 note(s)"), "{topics}");

    // filter by topic.
    let nut = ops::note_list(&db, Some("Nutrition")).unwrap();
    assert!(nut.contains("Protein timing"), "{nut}");
    assert!(!nut.contains("Squat depth"), "{nut}");

    // default note list shows notes but not the topic neurons themselves.
    let all = ops::note_list(&db, None).unwrap();
    assert!(all.contains("Protein timing") && all.contains("Squat depth"), "{all}");
    assert!(!all.contains("Nutrition"), "topics aren't notes: {all}");

    // untag clears the membership.
    ops::note_untag(&mut db, &protein[..8], "Nutrition").unwrap();
    assert!(ops::note_list(&db, Some("Nutrition")).unwrap().contains("no notes tagged"));

    // stats separates notes from topics (2 notes, 2 topics).
    let s = ops::stats(&db).unwrap();
    assert!(s.contains("notes:    2"), "{s}");
    assert!(s.contains("topics:   2"), "{s}");
}

#[test]
fn calendar_sync_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let ics_path = dir.path().join("cal.ics");
    let ics = "BEGIN:VCALENDAR\r\n\
               BEGIN:VEVENT\r\n\
               UID:evt-1\r\n\
               SUMMARY:James Bywater PT\r\n\
               DTSTART:20260613T090000Z\r\n\
               END:VEVENT\r\n\
               END:VCALENDAR\r\n";
    std::fs::write(&ics_path, ics).unwrap();

    let mut db = Db::open_in_memory().unwrap();
    ops::client_add(&mut db, "James Bywater", "active", None, None, NOW).unwrap();

    let first = ops::calendar_sync(&mut db, Some(&ics_path), NOW).unwrap();
    assert!(first.contains("1 matched"), "{first}");
    assert!(first.contains("1 new"), "{first}");

    let second = ops::calendar_sync(&mut db, Some(&ics_path), NOW).unwrap();
    assert!(second.contains("0 new"), "re-sync must not duplicate: {second}");
}
