//! Schema migrations.
//!
//! Each migration is a SQL file in `migrations/`, applied in order. SQLite's
//! `user_version` pragma records how many have run, so opening an old library
//! upgrades it in place.
//!
//! **Never edit a migration that has shipped.** Add a new file instead.

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::DbError;

/// Brings the schema of `conn` up to the latest version.
pub fn run(conn: &mut Connection) -> Result<(), DbError> {
    migrations().to_latest(conn)?;
    Ok(())
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../migrations/0001_songs.sql"))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }
}
