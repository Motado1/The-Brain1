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
}
