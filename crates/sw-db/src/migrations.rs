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
