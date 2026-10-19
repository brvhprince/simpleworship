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

mod bibles;
mod error;
mod fts;
mod media;
mod migrations;
mod presentations;
mod settings;
mod songs;
mod themes;
mod usage;

use std::path::Path;

use rusqlite::Connection;

pub use bibles::Bibles;
pub use error::DbError;
pub use media::Media;
pub use presentations::Presentations;
pub use settings::Settings;
pub use songs::Songs;
pub use themes::Themes;
pub use usage::Usage;

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
        // Another connection may be writing (a Bible import runs on its own
        // connection); wait for it instead of failing straight away.
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        migrations::run(&mut conn)?;
        let database = Self { conn };
        database.themes().add_starters()?;
        Ok(database)
    }

    /// Access to the song library.
    pub fn songs(&self) -> Songs<'_> {
        Songs::new(&self.conn)
    }

    /// Access to the installed Bibles.
    pub fn bibles(&self) -> Bibles<'_> {
        Bibles::new(&self.conn)
    }

    /// Access to the theme library.
    pub fn themes(&self) -> Themes<'_> {
        Themes::new(&self.conn)
    }

    /// Access to the media library's records.
    pub fn media(&self) -> Media<'_> {
        Media::new(&self.conn)
    }

    /// Access to imported presentations.
    pub fn presentations(&self) -> Presentations<'_> {
        Presentations::new(&self.conn)
    }

    /// Access to the CCLI song usage log.
    pub fn usage(&self) -> Usage<'_> {
        Usage::new(&self.conn)
    }

    /// Access to application settings.
    pub fn settings(&self) -> Settings<'_> {
        Settings::new(&self.conn)
    }
}
