//! The song library: create, read, update, delete and search.

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};
use sw_core::time;
use sw_core::{Id, Song, SongSummary};

use crate::DbError;

/// The most results a search or listing returns. Plenty for a picker list,
/// and it keeps the UI responsive on libraries with thousands of songs.
const RESULT_LIMIT: u32 = 500;

/// Access to the songs table. Get one from [`Database::songs`](crate::Database::songs).
pub struct Songs<'db> {
    conn: &'db Connection,
}

impl<'db> Songs<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// Lists all songs, sorted by title.
    pub fn list(&self) -> Result<Vec<SongSummary>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT id, title, author, lyrics
             FROM songs
             WHERE deleted_at IS NULL
             ORDER BY title COLLATE NOCASE
             LIMIT ?1",
        )?;
        let rows = statement.query_map([RESULT_LIMIT], |row| {
            let lyrics: String = row.get(3)?;
            Ok(SongSummary {
                id: read_id(row, 0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                excerpt: first_lyric_line(&lyrics),
            })
        })?;

        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Finds songs whose title, author or lyrics contain every word of
    /// `query`. Words match by prefix, so `"amaz gra"` finds "Amazing Grace".
    ///
    /// Title matches rank above author matches, which rank above lyric
    /// matches. An empty query lists every song (see [`Songs::list`]).
    pub fn search(&self, query: &str) -> Result<Vec<SongSummary>, DbError> {
        let Some(fts_query) = to_fts_query(query) else {
            return self.list();
        };

        let mut statement = self.conn.prepare_cached(
            "SELECT s.id, s.title, s.author,
                    snippet(songs_fts, 2, '', '', '…', 12)
             FROM songs_fts
             JOIN songs s ON s.seq = songs_fts.rowid
             WHERE songs_fts MATCH ?1 AND s.deleted_at IS NULL
             ORDER BY bm25(songs_fts, 10.0, 3.0, 1.0)
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![fts_query, RESULT_LIMIT], |row| {
            let snippet: String = row.get(3)?;
            Ok(SongSummary {
                id: read_id(row, 0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                excerpt: clean_excerpt(&snippet),
            })
        })?;

        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Loads one song. Returns `None` if it does not exist or was deleted.
    pub fn get(&self, id: Id) -> Result<Option<Song>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT id, title, author, copyright, ccli_number, lyrics, arrangement,
                    created_at, updated_at
             FROM songs
             WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let song = statement
            .query_row([id.to_string()], read_song)
            .optional()?;
        Ok(song)
    }

    /// Inserts a new song or updates an existing one (matched by id), and
    /// returns it with its timestamps filled in.
    ///
    /// Saving a song that was deleted brings it back.
    pub fn save(&self, song: &Song) -> Result<Song, DbError> {
        let now = time::now();
        let mut saved = song.clone();
        saved.updated_at = now;
        if saved.created_at == 0 {
            saved.created_at = now;
        }

        self.conn
            .prepare_cached(
                "INSERT INTO songs (id, title, author, copyright, ccli_number, lyrics,
                                    arrangement, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT (id) DO UPDATE SET
                     title = excluded.title,
                     author = excluded.author,
                     copyright = excluded.copyright,
                     ccli_number = excluded.ccli_number,
                     lyrics = excluded.lyrics,
                     arrangement = excluded.arrangement,
                     updated_at = excluded.updated_at,
                     deleted_at = NULL",
            )?
            .execute(params![
                saved.id.to_string(),
                saved.title,
                saved.author,
                saved.copyright,
                saved.ccli_number,
                saved.lyrics,
                saved.arrangement,
                saved.created_at,
                saved.updated_at,
            ])?;

        Ok(saved)
    }

    /// Marks a song as deleted. It disappears from listings and searches.
    /// Returns `false` if there was no such song.
    pub fn delete(&self, id: Id) -> Result<bool, DbError> {
        let now = time::now();
        let changed = self
            .conn
            .prepare_cached(
                "UPDATE songs SET deleted_at = ?2, updated_at = ?2
                 WHERE id = ?1 AND deleted_at IS NULL",
            )?
            .execute(params![id.to_string(), now])?;
        Ok(changed > 0)
    }
}

/// Reads a full [`Song`] from a row selected with the columns in `get`.
fn read_song(row: &Row<'_>) -> rusqlite::Result<Song> {
    Ok(Song {
        id: read_id(row, 0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        copyright: row.get(3)?,
        ccli_number: row.get(4)?,
        lyrics: row.get(5)?,
        arrangement: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

/// Reads an [`Id`] stored as text. A malformed id is reported as a normal
/// rusqlite conversion error.
fn read_id(row: &Row<'_>, column: usize) -> rusqlite::Result<Id> {
    let text: String = row.get(column)?;
    text.parse()
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

/// Turns what the operator typed into an FTS5 query: every word must match
/// as a prefix. Returns `None` if there are no searchable words.
///
/// Each word is quoted, so characters that mean something in FTS5 syntax
/// (`-`, `*`, `"`, `AND`) are treated as plain text.
fn to_fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|word| word.trim_matches('\''))
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{}\"*", word.replace('"', "")))
        .collect();

    (!terms.is_empty()).then(|| terms.join(" "))
}

/// The first line of actual lyrics, skipping section tags and blank lines.
fn first_lyric_line(lyrics: &str) -> String {
    lyrics
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('['))
        .unwrap_or_default()
        .to_owned()
}

/// Flattens an FTS snippet to one readable line: drops section tags and
/// joins lines with " / ".
fn clean_excerpt(snippet: &str) -> String {
    snippet
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !(line.starts_with('[') && line.ends_with(']')))
        .collect::<Vec<_>>()
        .join(" / ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn song(title: &str, author: &str, lyrics: &str) -> Song {
        let mut song = Song::new(title);
        song.author = author.into();
        song.lyrics = lyrics.into();
        song
    }

    fn titles(results: &[SongSummary]) -> Vec<&str> {
        results.iter().map(|s| s.title.as_str()).collect()
    }

    fn sample_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let songs = db.songs();
        songs
            .save(&song(
                "Amazing Grace",
                "John Newton",
                "[Verse 1]\nAmazing grace, how sweet the sound\nThat saved a wretch like me",
            ))
            .unwrap();
        songs
            .save(&song(
                "How Great Thou Art",
                "Carl Boberg",
                "[Verse 1]\nO Lord my God, when I in awesome wonder",
            ))
            .unwrap();
        songs
            .save(&song("Señor, Ten Piedad", "", "[Coro]\nSeñor ten piedad"))
            .unwrap();
        db
    }

    #[test]
    fn save_then_get_round_trips() {
        let db = Database::open_in_memory().unwrap();
        let original = song("Test", "Someone", "[V1]\nHello");

        let saved = db.songs().save(&original).unwrap();
        assert!(saved.created_at > 0);
        assert_eq!(saved.created_at, saved.updated_at);

        let loaded = db.songs().get(original.id).unwrap().unwrap();
        assert_eq!(loaded, saved);
    }

    #[test]
    fn saving_again_updates_in_place() {
        let db = Database::open_in_memory().unwrap();
        let mut song = db.songs().save(&song("Old Title", "", "")).unwrap();

        song.title = "New Title".into();
        db.songs().save(&song).unwrap();

        assert_eq!(titles(&db.songs().list().unwrap()), ["New Title"]);
        assert_eq!(titles(&db.songs().search("new").unwrap()), ["New Title"]);
        assert!(
            db.songs().search("old").unwrap().is_empty(),
            "index was updated"
        );
    }

    #[test]
    fn list_is_sorted_and_shows_first_lines() {
        let db = sample_db();
        let list = db.songs().list().unwrap();

        assert_eq!(
            titles(&list),
            ["Amazing Grace", "How Great Thou Art", "Señor, Ten Piedad"]
        );
        assert_eq!(list[0].excerpt, "Amazing grace, how sweet the sound");
    }

    #[test]
    fn search_matches_prefixes_across_fields() {
        let db = sample_db();
        let songs = db.songs();

        assert_eq!(
            titles(&songs.search("amaz gra").unwrap()),
            ["Amazing Grace"]
        );
        assert_eq!(
            titles(&songs.search("boberg").unwrap()),
            ["How Great Thou Art"]
        );
        assert_eq!(titles(&songs.search("wretch").unwrap()), ["Amazing Grace"]);
        assert!(
            songs.search("wretch zebra").unwrap().is_empty(),
            "all words must match"
        );
    }

    #[test]
    fn search_ignores_accents_and_fts_syntax() {
        let db = sample_db();
        let songs = db.songs();

        assert_eq!(
            titles(&songs.search("senor").unwrap()),
            ["Señor, Ten Piedad"]
        );
        assert_eq!(
            titles(&songs.search("\"grace* -(").unwrap()),
            ["Amazing Grace"],
            "FTS operators are treated as plain words"
        );
        assert_eq!(
            songs.search("   ").unwrap().len(),
            3,
            "blank query lists everything"
        );
    }

    #[test]
    fn search_excerpt_shows_matching_lyrics() {
        let db = sample_db();
        let hits = db.songs().search("wretch").unwrap();
        assert!(
            hits[0].excerpt.contains("wretch"),
            "excerpt was {:?}",
            hits[0].excerpt
        );
        assert!(!hits[0].excerpt.contains('['), "section tags are removed");
    }

    #[test]
    fn deleted_songs_disappear_and_can_be_restored() {
        let db = sample_db();
        let songs = db.songs();
        let grace = songs.search("amazing").unwrap()[0].id;

        assert!(songs.delete(grace).unwrap());
        assert!(!songs.delete(grace).unwrap(), "already deleted");
        assert_eq!(songs.get(grace).unwrap(), None);
        assert!(songs.search("amazing").unwrap().is_empty());

        let mut restored = Song::new("Amazing Grace");
        restored.id = grace;
        songs.save(&restored).unwrap();
        assert!(songs.get(grace).unwrap().is_some());
    }

    #[test]
    fn fts_query_quotes_every_word() {
        assert_eq!(
            to_fts_query("amazing grace").unwrap(),
            "\"amazing\"* \"grace\"*"
        );
        assert_eq!(to_fts_query("don't").unwrap(), "\"don't\"*");
        assert_eq!(to_fts_query(" - * "), None);
    }
}
