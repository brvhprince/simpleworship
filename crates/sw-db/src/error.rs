//! The error type for all database operations.

use thiserror::Error;

/// Something went wrong while reading or writing the library.
#[derive(Debug, Error)]
pub enum DbError {
    /// An error reported by SQLite.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// The schema could not be created or upgraded.
    #[error("could not upgrade the database schema: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    /// A stored JSON value (a theme or a setting) could not be read.
    #[error("stored data is damaged: {0}")]
    Json(#[from] serde_json::Error),
}
