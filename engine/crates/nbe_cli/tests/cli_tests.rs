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
    ops::slot_add(&mut db, &cid[..8], "Tue", "09:00", 60, 1.0).unwrap();
    ops::slot_add(&mut db, &cid[..8], "Thu", "09:00", 30, 1.0).unwrap();

    let h = ops::report_hours(&db).unwrap();
    assert!(h.contains("Weekly work hours: 1.5"), "{h}");
    assert!(h.contains("Tue  1.0h"), "{h}");
    assert!(h.contains("Thu  0.5h"), "{h}");
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
