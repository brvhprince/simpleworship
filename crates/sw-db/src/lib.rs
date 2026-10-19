//! # sw-db
//!
//! Persistent storage for the SimpleWorship library, backed by a single
//! SQLite file.
//!
//! ```no_run
//! # fn main() -> Result<(), sw_db::DbError> {
//! let db = sw_db::Database::open("library.sqlite")?;
//! let hits = db.songs().search("amazing gra")?;
//! # Ok(()) }
//! ```
//!
//! The schema is created and upgraded automatically by [`Database::open`]
//! (see the SQL files in `migrations/`).

mod error;
mod migrations;
mod songs;

use std::path::Path;

use rusqlite::Connection;

pub use error::DbError;
pub use songs::Songs;

/// An open library database.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens (or creates) the database file at `path` and brings its schema
    /// up to date.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        // WAL lets the UI read while a long import is writing.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    /// Opens a temporary database that lives only in memory. Handy for tests.
    pub fn open_in_memory() -> Result<Self, DbError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self, DbError> {
        conn.pragma_update(None, "foreign_keys", true)?;
        migrations::run(&mut conn)?;
        Ok(Self { conn })
    }

    /// Access to the song library.
    pub fn songs(&self) -> Songs<'_> {
        Songs::new(&self.conn)
    }
}
