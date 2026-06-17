//! `nbe_data` — the local-first data engine for the Neural Business Engine.
//!
//! A single-file SQLite database with optional SQLCipher AES-256 encryption at rest. This crate
//! is deliberately GPU-free and engine-free so it is fully testable headlessly (`cargo test`);
//! the Bevy app links it as a plain library.
//!
//! ```no_run
//! use nbe_data::Db;
//! let mut db = Db::open("brain.db", Some("correct horse battery staple")).unwrap();
//! nbe_data::seed::seed(&mut db, &Default::default()).unwrap();
//! ```

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

pub mod model;
pub mod repo;
mod schema;
pub mod seed;
pub mod snapshot;

pub use model::*;

/// Crate error type.
#[derive(Debug, Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Msg(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Generate a fresh random id for a new entity or edge.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// An open database handle. `conn` is public so repository/seed/snapshot functions (which take
/// `&Connection`) can be called directly.
pub struct Db {
    pub conn: Connection,
}

impl Db {
    /// Open (creating if needed) a database at `path`. When `passphrase` is `Some`, the file is
    /// encrypted with SQLCipher; when `None`, it is a plain SQLite file. Opening an encrypted
    /// file with a wrong/absent key returns an error (surfaced by the first schema read).
    pub fn open(path: impl AsRef<Path>, passphrase: Option<&str>) -> Result<Db> {
        let conn = Connection::open(path)?;
        Self::init(conn, passphrase)
    }

    /// In-memory database for tests. Never encrypted.
    pub fn open_in_memory() -> Result<Db> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn, None)
    }

    fn init(conn: Connection, passphrase: Option<&str>) -> Result<Db> {
        if let Some(pass) = passphrase {
            // SQLCipher: the key must be set before any other database access. We escape single
            // quotes and let SQLCipher derive the key with its native KDF (PBKDF2-HMAC-SHA512),
            // which keeps the salt inside the single file (preserving portability).
            let escaped = pass.replace('\'', "''");
            conn.execute_batch(&format!("PRAGMA key = '{escaped}';"))?;
        }

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        // WAL is invalid for in-memory databases; ignore the failure there.
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL;");

        schema::migrate(&conn)?;
        Ok(Db { conn })
    }

    /// Compact, single-file export (`VACUUM INTO`). The destination inherits the current
    /// encryption key, so an encrypted source produces an encrypted backup.
    pub fn vacuum_into(&self, path: impl AsRef<Path>) -> Result<()> {
        let dest = path.as_ref().to_string_lossy().replace('\'', "''");
        self.conn
            .execute_batch(&format!("VACUUM INTO '{dest}';"))?;
        Ok(())
    }

    /// Run a multi-step mutation atomically inside a single SQLite transaction: the closure receives
    /// a [`rusqlite::Transaction`] (which derefs to `&Connection`, so `repo::*` work unchanged). On
    /// `Ok` the transaction commits; on **any** `Err` it is dropped and SQLite rolls back, so a
    /// partially-applied batch can never be persisted.
    pub fn transact<T>(
        &mut self,
        f: impl FnOnce(&rusqlite::Transaction) -> Result<T>,
    ) -> Result<T> {
        let tx = self.conn.transaction()?;
        let value = f(&tx)?; // any Err returns here → `tx` drops → automatic ROLLBACK
        tx.commit()?;
        Ok(value)
    }

    /// Verify the database is structurally sound: `PRAGMA integrity_check` reports `ok` and
    /// `PRAGMA foreign_key_check` finds no dangling references. Errors describe the first problem.
    pub fn integrity_check(&self) -> Result<()> {
        let problems: Vec<String> = self
            .conn
            .prepare("PRAGMA integrity_check;")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?
            .into_iter()
            .filter(|s| s != "ok")
            .collect();
        if !problems.is_empty() {
            return Err(Error::Msg(format!("integrity check failed: {}", problems.join("; "))));
        }

        let mut stmt = self.conn.prepare("PRAGMA foreign_key_check;")?;
        let mut rows = stmt.query([])?;
        let mut violations = 0i64;
        while rows.next()?.is_some() {
            violations += 1;
        }
        if violations > 0 {
            return Err(Error::Msg(format!("{violations} foreign-key violation(s)")));
        }
        Ok(())
    }

    /// Integrity-checked backup: validate the live DB, then write a verified single-file copy. Use
    /// before risky batch work so there is always a known-good snapshot to fall back to.
    pub fn checked_backup(&self, path: impl AsRef<Path>) -> Result<()> {
        self.integrity_check()?;
        self.vacuum_into(path)
    }
}

#[cfg(test)]
mod integrity_tests {
    use super::*;

    fn entity(id: &str) -> Entity {
        Entity { id: id.to_string(), created_at: 0, updated_at: 0 }
    }

    #[test]
    fn transact_commits_on_success() {
        let mut db = Db::open_in_memory().unwrap();
        db.transact(|tx| repo::insert_entity(tx, &entity("keep-me"))).unwrap();
        assert!(repo::all_entity_ids(&db.conn).unwrap().iter().any(|id| id == "keep-me"));
    }

    #[test]
    fn transact_rolls_back_on_error() {
        let mut db = Db::open_in_memory().unwrap();
        let before = repo::count_entities(&db.conn).unwrap();
        // Write a row, then fail later in the same batch — nothing should persist.
        let result: Result<()> = db.transact(|tx| {
            repo::insert_entity(tx, &entity("ghost"))?;
            Err(Error::Msg("simulated mid-batch failure".into()))
        });
        assert!(result.is_err());
        assert_eq!(repo::count_entities(&db.conn).unwrap(), before, "batch must roll back");
        assert!(!repo::all_entity_ids(&db.conn).unwrap().iter().any(|id| id == "ghost"));
    }

    #[test]
    fn transact_rolls_back_on_sqlite_error() {
        let mut db = Db::open_in_memory().unwrap();
        let before = repo::count_entities(&db.conn).unwrap();
        // A genuine SQLite error (bad SQL) mid-batch must roll back the earlier valid insert.
        let result: Result<()> = db.transact(|tx| {
            repo::insert_entity(tx, &entity("fresh"))?;
            tx.execute("INSERT INTO no_such_table(x) VALUES (1)", [])?;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            repo::count_entities(&db.conn).unwrap(),
            before,
            "no partial insert survives a SQLite error"
        );
    }

    #[test]
    fn integrity_check_passes_on_healthy_db() {
        let mut db = Db::open_in_memory().unwrap();
        db.transact(|tx| repo::insert_entity(tx, &entity("a"))).unwrap();
        db.integrity_check().expect("a healthy db passes");
    }

    #[test]
    fn checked_backup_produces_a_valid_copy() {
        let mut db = Db::open_in_memory().unwrap();
        db.transact(|tx| repo::insert_entity(tx, &entity("backed-up"))).unwrap();

        let path = std::env::temp_dir().join(format!("nbe_backup_{}.db", new_id()));
        db.checked_backup(&path).unwrap();

        let restored = Db::open(&path, None).unwrap();
        restored.integrity_check().unwrap();
        assert!(repo::all_entity_ids(&restored.conn).unwrap().iter().any(|id| id == "backed-up"));

        let _ = std::fs::remove_file(&path);
    }
}
