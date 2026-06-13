//! P1 acceptance tests for the data engine. All run headlessly (no GPU).

use std::collections::HashSet;

use nbe_data::seed::{seed, SeedConfig};
use nbe_data::{model::*, repo, snapshot, Db};

fn entity(id: &str) -> Entity {
    Entity {
        id: id.into(),
        created_at: 1_700_000_000,
        updated_at: 1_700_000_000,
    }
}

// 1) Schema applies: all tables + the roll-up view exist.
#[test]
fn schema_applies() {
    let db = Db::open_in_memory().unwrap();
    let mut stmt = db
        .conn
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','view')")
        .unwrap();
    let names: HashSet<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    for expected in [
        "entity",
        "crm_facet",
        "ledger_facet",
        "knowledge_facet",
        "edge",
        "activation",
        "layer_assignment",
        "v_ledger_rollup",
    ] {
        assert!(names.contains(expected), "missing schema object: {expected}");
    }
}

// 2) Encryption: encrypted file is unreadable without the key, intact with it; plain works too.
#[test]
fn encryption_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secret.db");
    let pass = "correct horse battery staple";

    {
        let mut db = Db::open(&path, Some(pass)).unwrap();
        seed(&mut db, &SeedConfig { entities: 20, edges: 30, ..Default::default() }).unwrap();
    } // connection closes on drop

    // Wrong/absent key -> error.
    assert!(Db::open(&path, None).is_err(), "opened encrypted db without key");
    assert!(
        Db::open(&path, Some("wrong")).is_err(),
        "opened encrypted db with wrong key"
    );

    // Correct key -> data intact.
    let db = Db::open(&path, Some(pass)).unwrap();
    assert_eq!(repo::count_entities(&db.conn).unwrap(), 20);
    assert_eq!(repo::count_edges(&db.conn).unwrap(), 30);

    // A plain (unencrypted) database opens with no passphrase.
    let plain = dir.path().join("plain.db");
    {
        let _ = Db::open(&plain, None).unwrap();
    }
    assert!(Db::open(&plain, None).is_ok());
}

// 3) Each facet round-trips losslessly.
#[test]
fn facet_roundtrip() {
    let db = Db::open_in_memory().unwrap();
    repo::insert_entity(&db.conn, &entity("e1")).unwrap();

    let crm = CrmFacet {
        entity_id: "e1".into(),
        contact: Some("a@b.com".into()),
        lifecycle_stage: "active".into(),
        session_schedule: Some("weekly".into()),
        renewal_date: Some(1_800_000_000),
    };
    let ledger = LedgerFacet {
        entity_id: "e1".into(),
        amount_cents: 12_345,
        invoice_status: "paid".into(),
        is_expense: false,
        tax_bucket: Some("income".into()),
        pacing_target_cents: Some(50_000),
    };
    let knowledge = KnowledgeFacet {
        entity_id: "e1".into(),
        body_md: "# note".into(),
        template_type: Some("sop".into()),
        review_status: "reviewed".into(),
    };

    repo::upsert_crm(&db.conn, &crm).unwrap();
    repo::upsert_ledger(&db.conn, &ledger).unwrap();
    repo::upsert_knowledge(&db.conn, &knowledge).unwrap();

    assert_eq!(repo::get_crm(&db.conn, "e1").unwrap().unwrap(), crm);
    assert_eq!(repo::get_ledger(&db.conn, "e1").unwrap().unwrap(), ledger);
    assert_eq!(
        repo::get_knowledge(&db.conn, "e1").unwrap().unwrap(),
        knowledge
    );
}

// 4) Polymorphism: one entity carries all three facets at once.
#[test]
fn polymorphism() {
    let db = Db::open_in_memory().unwrap();
    repo::insert_entity(&db.conn, &entity("poly")).unwrap();
    repo::upsert_crm(
        &db.conn,
        &CrmFacet {
            entity_id: "poly".into(),
            contact: None,
            lifecycle_stage: "renewal".into(),
            session_schedule: None,
            renewal_date: None,
        },
    )
    .unwrap();
    repo::upsert_ledger(
        &db.conn,
        &LedgerFacet {
            entity_id: "poly".into(),
            amount_cents: 999,
            invoice_status: "sent".into(),
            is_expense: false,
            tax_bucket: None,
            pacing_target_cents: None,
        },
    )
    .unwrap();
    repo::upsert_knowledge(
        &db.conn,
        &KnowledgeFacet {
            entity_id: "poly".into(),
            body_md: "body".into(),
            template_type: None,
            review_status: "draft".into(),
        },
    )
    .unwrap();
    repo::set_layer(&db.conn, "poly", &Layer::Hidden(1)).unwrap();

    let ewf = repo::entity_with_facets(&db.conn, "poly").unwrap().unwrap();
    assert!(ewf.crm.is_some() && ewf.ledger.is_some() && ewf.knowledge.is_some());
    assert_eq!(ewf.layer, Some(Layer::Hidden(1)));
}

// 5) Backlinks: every edge pointing at a node is returned, and only those.
#[test]
fn backlinks() {
    let db = Db::open_in_memory().unwrap();
    for id in ["a", "b", "c", "target"] {
        repo::insert_entity(&db.conn, &entity(id)).unwrap();
    }
    let mk = |id: &str, s: &str, t: &str| Edge {
        id: id.into(),
        source_id: s.into(),
        target_id: t.into(),
        edge_type: "flow".into(),
        weight: 1.0,
        directed: true,
    };
    repo::insert_edge(&db.conn, &mk("e1", "a", "target")).unwrap();
    repo::insert_edge(&db.conn, &mk("e2", "b", "target")).unwrap();
    repo::insert_edge(&db.conn, &mk("e3", "c", "a")).unwrap(); // not a backlink of target

    let got: HashSet<String> = repo::backlinks(&db.conn, "target")
        .unwrap()
        .into_iter()
        .map(|e| e.source_id)
        .collect();
    assert_eq!(got, HashSet::from(["a".to_string(), "b".to_string()]));
}

// 6) Seeding is deterministic and hits exact counts; a different seed differs.
#[test]
fn seed_deterministic() {
    let cfg = SeedConfig::default();

    let mut a = Db::open_in_memory().unwrap();
    seed(&mut a, &cfg).unwrap();
    let mut b = Db::open_in_memory().unwrap();
    seed(&mut b, &cfg).unwrap();

    assert_eq!(repo::count_entities(&a.conn).unwrap(), 500);
    assert_eq!(repo::count_edges(&a.conn).unwrap(), 1200);

    let sa = snapshot::export(&a).unwrap();
    let sb = snapshot::export(&b).unwrap();
    assert_eq!(sa, sb, "same seed must produce identical databases");

    let mut c = Db::open_in_memory().unwrap();
    seed(&mut c, &SeedConfig { rng_seed: cfg.rng_seed + 1, ..cfg.clone() }).unwrap();
    let sc = snapshot::export(&c).unwrap();
    assert_ne!(sa.entities, sc.entities, "different seed should differ");
}

// 7) JSON snapshot round-trips idempotently.
#[test]
fn snapshot_idempotent() {
    let mut src = Db::open_in_memory().unwrap();
    seed(&mut src, &SeedConfig { entities: 60, edges: 120, ..Default::default() }).unwrap();

    let json = snapshot::export_json_string(&src).unwrap();

    let mut dst = Db::open_in_memory().unwrap();
    snapshot::import_json_string(&mut dst, &json).unwrap();

    let again = snapshot::export_json_string(&dst).unwrap();
    assert_eq!(json, again, "export -> import -> export must be stable");
}

// 8) Financial roll-up is internally consistent.
#[test]
fn ledger_rollup_consistent() {
    let mut db = Db::open_in_memory().unwrap();
    seed(&mut db, &SeedConfig::default()).unwrap();

    let r = repo::ledger_rollup(&db.conn).unwrap();
    assert_eq!(r.paid_cents + r.outstanding_cents, r.total_cents);
    assert_eq!(r.income_cents + r.expense_cents, r.total_cents);
    assert!(r.total_cents > 0);
}
