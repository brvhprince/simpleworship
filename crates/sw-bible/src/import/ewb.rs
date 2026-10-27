//! EasyWorship Bibles (`.ewb`).
//!
//! An `.ewb` file is a SQLite database:
//!
//! - `header`: `name`, `abbrev_name`, `lang_code`, `copyright`.
//! - `books`: one row per book; `rowid` is the book number (1 = Genesis).
//!   `verse_info` holds an 8-byte record per verse.
//! - `streams`: one row per book (same `rowid`); `stream` is the book's
//!   text, zlib-compressed, all verses run together after a 2-byte prefix.
//!
//! Each verse record is two little-endian 32-bit numbers:
//!
//! | Number | Bits   | Meaning                                  |
//! | ------ | ------ | ---------------------------------------- |
//! | first  | 0–9    | length of the verse text                 |
//! | first  | 10–31  | where it starts in the text              |
//! | second | 2–9    | verse number                             |
//! | second | 10–17  | chapter number                           |
//! | second | 18–24  | book number                              |
//!
//! EasyWorship counts the start and length in *characters*. Some
//! hand-made files count *bytes* instead, which only differs once the text
//! has non-ASCII letters (ɛ, ɔ, é…). For each book both readings are tried,
//! and the one whose verses end at sentence boundaries wins.

use std::io::Read;
use std::path::Path;

use flate2::read::ZlibDecoder;
use rusqlite::{Connection, OpenFlags};

use super::{BibleData, ImportError, VerseText};
use crate::books::BookId;

/// Bytes before the first verse in a decompressed book.
const PREFIX: usize = 2;

/// Characters a verse usually ends with; used to pick the right reading.
const VERSE_ENDINGS: &[char] = &[
    '.', '?', '!', ';', ':', ',', '"', '\'', '”', '’', ')', ']', '»',
];

/// One verse's position, as stored in `verse_info`.
#[derive(Clone, Copy)]
struct Record {
    start: usize,
    length: usize,
    book: u8,
    chapter: u16,
    verse: u16,
}

/// Reads an `.ewb` file.
pub fn read(path: &Path) -> Result<BibleData, ImportError> {
    let conn = open_read_only(path).map_err(not_a_bible)?;

    let mut bible = BibleData::default();
    conn.query_row(
        "SELECT name, abbrev_name, lang_code, copyright FROM header LIMIT 1",
        [],
        |row| {
            let text = |i| {
                row.get::<_, Option<String>>(i)
                    .map(Option::unwrap_or_default)
            };
            bible.name = text(0)?;
            bible.abbreviation = text(1)?;
            bible.language = text(2)?.to_lowercase();
            // EasyWorship keeps the notice as RTF: show its words only.
            bible.copyright = sw_rtf::plain_line(&text(3)?);
            Ok(())
        },
    )
    .map_err(not_a_bible)?;

    let mut statement = conn
        .prepare("SELECT b.rowid, b.verse_info, s.stream FROM books b JOIN streams s ON s.rowid = b.rowid")
        .map_err(not_a_bible)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(not_a_bible)?;

    for row in rows {
        let (_, verse_info, stream) = row.map_err(not_a_bible)?;
        let Some(text) = decompress(&stream) else {
            continue; // A damaged book is skipped; the rest still imports.
        };
        bible.verses.extend(read_book(&records(&verse_info), &text));
    }
    Ok(bible)
}

/// Decodes the 8-byte verse records.
fn records(verse_info: &[u8]) -> Vec<Record> {
    verse_info
        .as_chunks::<8>()
        .0
        .iter()
        .map(|&[p0, p1, p2, p3, r0, r1, r2, r3]| {
            let position = u32::from_le_bytes([p0, p1, p2, p3]);
            let reference = u32::from_le_bytes([r0, r1, r2, r3]);
            Record {
                start: (position >> 10) as usize,
                length: (position & 0x3FF) as usize,
                verse: ((reference >> 2) & 0xFF) as u16,
                chapter: ((reference >> 10) & 0xFF) as u16,
                book: ((reference >> 18) & 0x7F) as u8,
            }
        })
        .collect()
}

/// Cuts a book's text into verses, choosing characters or bytes as the unit.
fn read_book(records: &[Record], text: &[u8]) -> Vec<VerseText> {
    let as_text = String::from_utf8_lossy(text);
    let characters: Vec<char> = as_text.chars().collect();

    let by_characters = |record: &Record| -> String {
        let start = (PREFIX + record.start).min(characters.len());
        let end = (start + record.length).min(characters.len());
        characters[start..end].iter().collect()
    };
    let by_bytes = |record: &Record| -> String {
        let start = (PREFIX + record.start).min(text.len());
        let end = (start + record.length).min(text.len());
        String::from_utf8_lossy(&text[start..end]).into_owned()
    };

    let endings = |cut: &dyn Fn(&Record) -> String| {
        records
            .iter()
            .filter(|record| cut(record).trim_end().ends_with(VERSE_ENDINGS))
            .count()
    };
    // Equal for plain-ASCII text, where both readings are the same.
    let use_bytes = endings(&by_bytes) > endings(&by_characters);

    records
        .iter()
        .filter(|record| (1..=66).contains(&record.book) && record.chapter > 0 && record.verse > 0)
        .map(|record| VerseText {
            book: BookId(record.book),
            chapter: record.chapter,
            verse: record.verse,
            text: if use_bytes {
                by_bytes(record)
            } else {
                by_characters(record)
            },
        })
        .collect()
}

fn decompress(stream: &[u8]) -> Option<Vec<u8>> {
    let mut text = Vec::new();
    ZlibDecoder::new(stream).read_to_end(&mut text).ok()?;
    Some(text)
}

fn open_read_only(path: &Path) -> rusqlite::Result<Connection> {
    let uri = format!(
        "file:{}?immutable=1",
        path.display()
            .to_string()
            .replace('\\', "/")
            .replace('?', "%3f")
            .replace('#', "%23")
    );
    Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
}

fn not_a_bible(error: rusqlite::Error) -> ImportError {
    ImportError::Malformed {
        reason: format!("not an EasyWorship Bible ({error})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    /// Encodes a verse record the way EasyWorship does.
    fn record(start: usize, length: usize, book: u32, chapter: u32, verse: u32) -> Vec<u8> {
        let position = ((start as u32) << 10) | length as u32;
        let reference = (book << 18) | (chapter << 10) | (verse << 2);
        [position.to_le_bytes(), reference.to_le_bytes()].concat()
    }

    /// Writes a one-book `.ewb` whose offsets count characters or bytes.
    fn make_ewb(path: &Path, verses: &[&str], count_bytes: bool) {
        let mut text = vec![0x00, 0x11];
        let mut info = Vec::new();
        let mut position = 0;
        for (index, verse) in verses.iter().enumerate() {
            let piece = format!("{verse} ");
            let length = if count_bytes {
                piece.len()
            } else {
                piece.chars().count()
            };
            info.extend(record(position, length, 43, 3, 16 + index as u32));
            position += length;
            text.extend(piece.as_bytes());
        }
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&text).unwrap();
        let stream = encoder.finish().unwrap();

        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE header (rowid INTEGER PRIMARY KEY, id INT8, key INT8, name TEXT, abbrev_name TEXT,
                 lang_code TEXT, plugin_version INT8, format_version INT8, copyright TEXT, license TEXT);
             CREATE TABLE books (rowid INTEGER PRIMARY KEY, name TEXT, abbrev_name TEXT, alt_name TEXT,
                 book_info BLOB, verse_info BLOB);
             CREATE TABLE streams (rowid INTEGER PRIMARY KEY, stream BLOB);
             INSERT INTO header (name, abbrev_name, lang_code, copyright)
             VALUES ('Asante Twi', 'TWI', 'TW', 'Bible Society of Ghana');",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO books (rowid, name, abbrev_name, verse_info) VALUES (43, 'Yohane', 'Yoh', ?1)",
            [info],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO streams (rowid, stream) VALUES (43, ?1)",
            [stream],
        )
        .unwrap();
    }

    const VERSES: [&str; 2] = [
        "Na sɛnea Onyankopɔn dɔɔ wiase ni.",
        "Ɛfiri sɛ Onyankopɔn ansoma ne Ba.",
    ];

    fn texts(bible: &BibleData) -> Vec<&str> {
        bible.verses.iter().map(|v| v.text.trim()).collect()
    }

    #[test]
    fn reads_character_offsets_like_easyworship() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("TWI.ewb");
        make_ewb(&path, &VERSES, false);

        let bible = read(&path).unwrap();
        assert_eq!(bible.name, "Asante Twi");
        assert_eq!(bible.language, "tw");
        assert_eq!(texts(&bible), VERSES);
        assert_eq!((bible.verses[1].chapter, bible.verses[1].verse), (3, 17));
    }

    #[test]
    fn reads_byte_offsets_from_hand_made_files() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("TWI.ewb");
        make_ewb(&path, &VERSES, true);
        assert_eq!(texts(&read(&path).unwrap()), VERSES);
    }

    #[test]
    fn reads_a_copyright_kept_as_rtf_as_plain_words() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("HCSB.ewb");
        make_ewb(&path, &VERSES, false);
        Connection::open(&path)
            .unwrap()
            .execute(
                "UPDATE header SET copyright = ?1",
                [r"{\rtf1\ansi\deff0{\fonttbl{\f0\fnil Courier New;}}\pard\f0 Holman Bible Publishers\par}"],
            )
            .unwrap();
        assert_eq!(read(&path).unwrap().copyright, "Holman Bible Publishers");
    }
}
