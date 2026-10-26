//! Schema migrations.
//!
//! Each migration is a SQL file in `migrations/`, applied in order. SQLite's
//! `user_version` pragma records how many have run, so opening an old library
//! upgrades it in place.
//!
//! **Never edit a migration that has shipped.** Add a new file instead.

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations, SchemaVersion};

use crate::DbError;

/// Brings the schema of `conn` up to the latest version. A library a newer
/// SimpleWorship has upgraded past what this one knows is
/// [`DbError::NewerLibrary`], and is left exactly as it is.
pub fn run(conn: &mut Connection) -> Result<(), DbError> {
    migrations().to_latest(conn).map_err(|error| match error {
        rusqlite_migration::Error::MigrationDefinition(
            rusqlite_migration::MigrationDefinitionError::DatabaseTooFarAhead,
        ) => DbError::NewerLibrary,
        other => DbError::Migration(other),
    })
}

/// Fails with [`DbError::NewerLibrary`] if a newer SimpleWorship has
/// upgraded `conn` past what this one knows. Only reads, so it is checked
/// before anything else touches the file.
pub fn check_known(conn: &Connection) -> Result<(), DbError> {
    match migrations().current_version(conn)? {
        SchemaVersion::Outside(_) => Err(DbError::NewerLibrary),
        SchemaVersion::NoneSet | SchemaVersion::Inside(_) => Ok(()),
    }
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_songs.sql")),
        M::up(include_str!("../migrations/0002_bibles.sql")),
        M::up(include_str!("../migrations/0003_themes_media.sql")),
        M::up(include_str!(
            "../migrations/0004_audio_presentations_usage.sql"
        )),
        M::up(include_str!(
            "../migrations/0005_editable_presentations.sql"
        )),
        M::up(include_str!("../migrations/0006_media_scaling.sql")),
        M::up(include_str!("../migrations/0007_bible_sources.sql")),
        M::up(include_str!(
            "../migrations/0008_removed_bundled_bibles.sql"
        )),
        M::up(include_str!("../migrations/0009_footers_off.sql")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footers_are_turned_off_in_existing_themes() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations().to_version(&mut conn, 8).unwrap();
        conn.execute_batch(
            "INSERT INTO themes (id, theme, created_at, updated_at) VALUES
               ('01920c7e-0000-7000-8000-000000000001',
                '{\"name\":\"On\",\"footer\":{\"visible\":true},\"caption\":{\"visible\":true}}', 1, 1)",
        )
        .unwrap();
        migrations().to_latest(&mut conn).unwrap();
        let (footer, caption): (bool, bool) = conn
            .query_row(
                "SELECT json_extract(theme, '$.footer.visible'),
                        json_extract(theme, '$.caption.visible') FROM themes",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(!footer, "the footer is turned off");
        assert!(caption, "the scripture reference is left alone");
    }

    #[test]
    fn a_library_from_a_newer_version_is_refused_and_left_alone() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        // As if a newer SimpleWorship had added a migration.
        conn.pragma_update(None, "user_version", 999).unwrap();
        assert!(matches!(run(&mut conn), Err(DbError::NewerLibrary)));
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 999, "not touched");
    }

    #[test]
    fn a_newer_library_file_is_opened_without_changing_it() {
        let path = std::env::temp_dir().join(format!("sw-newer-{}.sqlite", std::process::id()));
        {
            let mut conn = Connection::open(&path).unwrap();
            run(&mut conn).unwrap();
            conn.pragma_update(None, "journal_mode", "DELETE").unwrap();
            conn.pragma_update(None, "user_version", 999).unwrap();
        }
        assert!(matches!(
            crate::Database::open(&path),
            Err(DbError::NewerLibrary)
        ));
        let conn = Connection::open(&path).unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        drop(conn);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(mode, "delete", "not switched to WAL");
    }

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }

    #[test]
    fn rebuilding_the_media_table_keeps_existing_media() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations().to_version(&mut conn, 3).unwrap();
        conn.execute_batch(
            "INSERT INTO media (id, kind, name, file, size_bytes, created_at)
             VALUES ('01920c7e-0000-7000-8000-000000000001', 'video', 'Loop', 'a.mp4', 5, 1)",
        )
        .unwrap();

        migrations().to_latest(&mut conn).unwrap();
        let (name, collection): (String, Option<String>) = conn
            .query_row("SELECT name, collection FROM media", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(name, "Loop");
        assert_eq!(collection, None);
        conn.execute_batch(
            "INSERT INTO media (id, kind, name, file, size_bytes, created_at)
             VALUES ('01920c7e-0000-7000-8000-000000000002', 'audio', 'Hymn', 'b.mp3', 5, 1)",
        )
        .expect("audio is allowed now");
    }
}
