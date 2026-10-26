//! Bible translations: importing, listing, looking up verses and searching.

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};
use sw_bible::import::BibleData;
use sw_bible::{BookId, Passage};
use sw_core::time;
use sw_core::{BibleInfo, BookOutline, Id, Verse};

use crate::DbError;
use crate::fts::to_fts_query;

/// The most results a verse search returns.
const SEARCH_LIMIT: u32 = 200;

/// Columns selected for a [`BibleInfo`], in the order [`read_bible`] expects.
const BIBLE_COLUMNS: &str = "b.id, b.name, b.abbreviation, b.language, b.copyright,
     b.bundled_key IS NOT NULL,
     (SELECT COUNT(*) FROM verses v WHERE v.bible_seq = b.seq)";

/// Access to the Bibles. Get one from [`Database::bibles`](crate::Database::bibles).
pub struct Bibles<'db> {
    conn: &'db Connection,
}

impl<'db> Bibles<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// Every installed Bible, sorted by abbreviation.
    pub fn list(&self) -> Result<Vec<BibleInfo>, DbError> {
        let sql =
            format!("SELECT {BIBLE_COLUMNS} FROM bibles b ORDER BY b.abbreviation COLLATE NOCASE");
        let mut statement = self.conn.prepare_cached(&sql)?;
        let rows = statement.query_map([], read_bible)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// One Bible, if it is installed.
    pub fn get(&self, id: Id) -> Result<Option<BibleInfo>, DbError> {
        let sql = format!("SELECT {BIBLE_COLUMNS} FROM bibles b WHERE b.id = ?1");
        let mut statement = self.conn.prepare_cached(&sql)?;
        Ok(statement
            .query_row([id.to_string()], read_bible)
            .optional()?)
    }

    /// Whether the bundled Bible with this key ("kjv") was already imported,
    /// even if it was later renamed.
    pub fn has_bundled(&self, key: &str) -> Result<bool, DbError> {
        let mut statement = self
            .conn
            .prepare_cached("SELECT 1 FROM bibles WHERE bundled_key = ?1")?;
        Ok(statement.exists([key])?)
    }

    /// Stores an imported Bible and returns it. Everything happens in one
    /// transaction: if anything fails, nothing is stored.
    ///
    /// `bundled_key` marks a Bible that ships with SimpleWorship.
    pub fn import(
        &self,
        bible: &BibleData,
        bundled_key: Option<&str>,
    ) -> Result<BibleInfo, DbError> {
        let id = Id::new();
        let now = time::now();
        let transaction = self.conn.unchecked_transaction()?;

        transaction
            .prepare_cached(
                "INSERT INTO bibles (id, name, abbreviation, language, copyright, bundled_key,
                                     created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            )?
            .execute(params![
                id.to_string(),
                bible.name,
                bible.abbreviation,
                bible.language,
                bible.copyright,
                bundled_key,
                now,
            ])?;
        let bible_seq = transaction.last_insert_rowid();

        {
            let mut insert = transaction.prepare_cached(
                "INSERT INTO verses (bible_seq, book, chapter, verse, text)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for verse in &bible.verses {
                insert.execute(params![
                    bible_seq,
                    verse.book.0,
                    verse.chapter,
                    verse.verse,
                    verse.text,
                ])?;
            }
        }
        transaction.commit()?;

        self.get(id)?
            .ok_or(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows))
    }

    /// Changes a Bible's name and abbreviation. Returns `false` if there is
    /// no such Bible.
    pub fn rename(&self, id: Id, name: &str, abbreviation: &str) -> Result<bool, DbError> {
        let changed = self
            .conn
            .prepare_cached(
                "UPDATE bibles SET name = ?2, abbreviation = ?3, updated_at = ?4 WHERE id = ?1",
            )?
            .execute(params![id.to_string(), name, abbreviation, time::now()])?;
        Ok(changed > 0)
    }

    /// Records where a Bible came from, e.g. `"ebible:twi"` for one
    /// downloaded from eBible.org. Returns `false` if there is no such Bible.
    pub fn set_source(&self, id: Id, source: &str) -> Result<bool, DbError> {
        let changed = self
            .conn
            .prepare_cached("UPDATE bibles SET source = ?2 WHERE id = ?1")?
            .execute(params![id.to_string(), source])?;
        Ok(changed > 0)
    }

    /// Records where the bundled Bible with this key came from, if that is
    /// not known yet (Bibles bundled before sources were kept).
    pub fn set_bundled_source(&self, key: &str, source: &str) -> Result<(), DbError> {
        self.conn
            .prepare_cached(
                "UPDATE bibles SET source = ?2 WHERE bundled_key = ?1 AND source IS NULL",
            )?
            .execute(params![key, source])?;
        Ok(())
    }

    /// Where the installed Bibles came from, for those that were downloaded
    /// (see [`Bibles::set_source`]), e.g. to mark them as installed in a
    /// catalogue.
    pub fn sources(&self) -> Result<Vec<String>, DbError> {
        let mut statement = self
            .conn
            .prepare_cached("SELECT source FROM bibles WHERE source IS NOT NULL")?;
        let rows = statement.query_map([], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Deletes a Bible and all its verses. Returns `false` if there was no
    /// such Bible.
    pub fn delete(&self, id: Id) -> Result<bool, DbError> {
        let changed = self
            .conn
            .prepare_cached("DELETE FROM bibles WHERE id = ?1")?
            .execute([id.to_string()])?;
        Ok(changed > 0)
    }

    /// Which books and chapters a Bible has, in canonical order.
    pub fn outline(&self, id: Id) -> Result<Vec<BookOutline>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT v.book, v.chapter, MAX(v.verse)
             FROM verses v JOIN bibles b ON b.seq = v.bible_seq
             WHERE b.id = ?1
             GROUP BY v.book, v.chapter
             ORDER BY v.book, v.chapter",
        )?;
        let rows = statement.query_map([id.to_string()], |row| {
            Ok((
                BookId(row.get(0)?),
                row.get::<_, u16>(1)?,
                row.get::<_, u16>(2)?,
            ))
        })?;

        let mut outline: Vec<BookOutline> = Vec::new();
        for row in rows {
            let (book, chapter, verses) = row?;
            if outline.last().is_none_or(|last| last.book != book) {
                outline.push(BookOutline {
                    book,
                    name: book.name(),
                    chapters: Vec::new(),
                });
            }
            let chapters = &mut outline.last_mut().expect("just pushed").chapters;
            // Fill any chapter missing from the file with zero verses, so
            // `chapters[n - 1]` is always chapter n.
            chapters.resize(usize::from(chapter) - 1, 0);
            chapters.push(verses);
        }
        Ok(outline)
    }

    /// The verses of a passage, in order. Verses the Bible does not have are
    /// simply missing from the result.
    pub fn passage(&self, id: Id, passage: &Passage) -> Result<Vec<Verse>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT v.book, v.chapter, v.verse, v.text
             FROM verses v JOIN bibles b ON b.seq = v.bible_seq
             WHERE b.id = ?1 AND v.book = ?2
               AND (v.chapter, v.verse) BETWEEN (?3, ?4) AND (?5, ?6)
             ORDER BY v.chapter, v.verse",
        )?;

        let mut verses: Vec<Verse> = Vec::new();
        for range in &passage.ranges {
            // A whole chapter runs from verse 0 to "as far as it goes".
            let rows = statement.query_map(
                params![
                    id.to_string(),
                    passage.book.0,
                    range.start.chapter,
                    range.start.verse.unwrap_or(0),
                    range.end.chapter,
                    range.end.verse.unwrap_or(u16::MAX),
                ],
                read_verse,
            )?;
            for verse in rows {
                let verse = verse?;
                // Overlapping ranges ("3:16-18, 17") list each verse once.
                let seen = verses
                    .iter()
                    .any(|v| (v.chapter, v.verse) == (verse.chapter, verse.verse));
                if !seen {
                    verses.push(verse);
                }
            }
        }
        Ok(verses)
    }

    /// Verses containing every word of `query` (as word prefixes), best
    /// matches first.
    pub fn search(&self, id: Id, query: &str) -> Result<Vec<Verse>, DbError> {
        let Some(fts_query) = to_fts_query(query) else {
            return Ok(Vec::new());
        };
        let mut statement = self.conn.prepare_cached(
            "SELECT v.book, v.chapter, v.verse, v.text
             FROM verses_fts
             JOIN verses v ON v.seq = verses_fts.rowid
             JOIN bibles b ON b.seq = v.bible_seq
             WHERE verses_fts MATCH ?1 AND b.id = ?2
             ORDER BY rank
             LIMIT ?3",
        )?;
        let rows =
            statement.query_map(params![fts_query, id.to_string(), SEARCH_LIMIT], read_verse)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn read_bible(row: &Row<'_>) -> rusqlite::Result<BibleInfo> {
    let id: String = row.get(0)?;
    Ok(BibleInfo {
        id: id.parse().map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
        })?,
        name: row.get(1)?,
        abbreviation: row.get(2)?,
        language: row.get(3)?,
        copyright: row.get(4)?,
        bundled: row.get(5)?,
        verse_count: row.get(6)?,
    })
}

fn read_verse(row: &Row<'_>) -> rusqlite::Result<Verse> {
    Ok(Verse {
        book: BookId(row.get(0)?),
        chapter: row.get(1)?,
        verse: row.get(2)?,
        text: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use sw_bible::import::VerseText;
    use sw_bible::reference::parse;

    fn sample() -> BibleData {
        let verse = |book: u8, chapter: u16, verse: u16, text: &str| VerseText {
            book: BookId(book),
            chapter,
            verse,
            text: text.into(),
        };
        BibleData {
            name: "Test Bible".into(),
            abbreviation: "TB".into(),
            language: "en".into(),
            copyright: "Public Domain".into(),
            verses: vec![
                verse(43, 3, 15, "whoever believes may have eternal life."),
                verse(43, 3, 16, "For God so loved the world."),
                verse(43, 3, 17, "For God did not send his Son to condemn."),
                verse(43, 4, 1, "Jesus learned of it."),
                verse(1, 1, 1, "In the beginning God created."),
            ],
        }
    }

    fn numbers(verses: &[Verse]) -> Vec<(u16, u16)> {
        verses.iter().map(|v| (v.chapter, v.verse)).collect()
    }

    #[test]
    fn a_downloaded_bible_remembers_where_it_came_from() {
        let db = Database::open_in_memory().unwrap();
        let bibles = db.bibles();
        let info = bibles.import(&sample(), None).unwrap();
        assert!(bibles.sources().unwrap().is_empty());

        assert!(bibles.set_source(info.id, "ebible:twi").unwrap());
        assert_eq!(bibles.sources().unwrap(), ["ebible:twi"]);

        bibles.delete(info.id).unwrap();
        assert!(bibles.sources().unwrap().is_empty(), "deleting forgets it");
    }

    #[test]
    fn import_list_rename_delete() {
        let db = Database::open_in_memory().unwrap();
        let bibles = db.bibles();

        let info = bibles.import(&sample(), Some("test")).unwrap();
        assert_eq!(info.verse_count, 5);
        assert!(info.bundled);
        assert!(bibles.has_bundled("test").unwrap());
        assert_eq!(bibles.list().unwrap(), std::slice::from_ref(&info));

        assert!(bibles.rename(info.id, "Renamed", "RN").unwrap());
        assert_eq!(bibles.get(info.id).unwrap().unwrap().abbreviation, "RN");

        assert!(bibles.delete(info.id).unwrap());
        assert!(bibles.list().unwrap().is_empty());
        assert!(
            bibles.search(info.id, "God").unwrap().is_empty(),
            "verses went too"
        );
    }

    #[test]
    fn looks_up_passages() {
        let db = Database::open_in_memory().unwrap();
        let id = db.bibles().import(&sample(), None).unwrap().id;
        let passage = |text: &str| {
            db.bibles()
                .passage(id, &parse(text).unwrap().passage)
                .unwrap()
        };

        assert_eq!(numbers(&passage("jn 3:16")), [(3, 16)]);
        assert_eq!(numbers(&passage("jn 3:16-4:1")), [(3, 16), (3, 17), (4, 1)]);
        assert_eq!(numbers(&passage("jn 3")), [(3, 15), (3, 16), (3, 17)]);
        assert_eq!(numbers(&passage("jn 3:15, 17")), [(3, 15), (3, 17)]);
        assert_eq!(
            numbers(&passage("jn 3:15-17, 16")),
            [(3, 15), (3, 16), (3, 17)]
        );
        assert!(passage("jn 9:1").is_empty());
    }

    #[test]
    fn outlines_books_and_chapters() {
        let db = Database::open_in_memory().unwrap();
        let id = db.bibles().import(&sample(), None).unwrap().id;
        let outline = db.bibles().outline(id).unwrap();

        assert_eq!(outline.len(), 2);
        assert_eq!(outline[0].name, "Genesis");
        assert_eq!(outline[1].book, BookId(43));
        // John 1 and 2 are not in the sample, so they count zero verses.
        assert_eq!(outline[1].chapters, [0, 0, 17, 1]);
    }

    #[test]
    fn searches_words() {
        let db = Database::open_in_memory().unwrap();
        let id = db.bibles().import(&sample(), None).unwrap().id;

        let hits = db.bibles().search(id, "god lov").unwrap();
        assert_eq!(numbers(&hits), [(3, 16)]);
        assert_eq!(db.bibles().search(id, "god").unwrap().len(), 3);
        assert!(db.bibles().search(id, "  ").unwrap().is_empty());
    }
}
