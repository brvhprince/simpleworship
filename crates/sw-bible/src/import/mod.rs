//! Importing Bibles from the common open file formats.
//!
//! | Format   | Typical file                         | Module        |
//! | -------- | ------------------------------------ | ------------- |
//! | OSIS     | one `.xml` / `.osis` file            | [`osis`]      |
//! | Zefania  | one `.xml` file (`<XMLBIBLE>`)       | [`zefania`]   |
//! | OpenSong | one `.xmm` / `.xml` file (`<bible>`) | [`opensong`]  |
//! | USX      | a `.zip` of one `.usx` file per book | [`usx`]       |
//! | USFM     | a `.zip` of one `.usfm` / `.sfm` per book | [`usfm`] |
//! | EasyWorship | one `.ewb` file (SQLite)             | [`ewb`]       |
//!
//! [`import_file`] reads a file from disk and works out its format by
//! looking inside; [`import`] does the same for bytes already in memory
//! (all formats except `.ewb`, which is a database). Both return a
//! [`BibleData`]: the verse texts with footnotes, cross-references, Strong's
//! numbers and headings removed.
//!
//! Real-world files are messy. Importers keep what they understand and skip
//! the rest instead of failing, and only report an error when no verses at
//! all could be found.

pub mod ewb;
pub mod opensong;
pub mod osis;
pub mod usfm;
pub mod usx;
mod xml;
pub mod zefania;

use std::io::{Cursor, Read};

use thiserror::Error;

use crate::books::BookId;

/// A Bible read from a file, ready to be stored.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BibleData {
    /// Full name, e.g. "King James Version". May be empty if the file does
    /// not say; the import dialog lets the operator fill it in.
    pub name: String,
    /// Short name shown in the UI, e.g. "KJV". May be empty.
    pub abbreviation: String,
    /// Language code or name, e.g. "en". May be empty.
    pub language: String,
    /// Copyright or licence notice. May be empty.
    pub copyright: String,
    /// Every verse, sorted by book, chapter and verse.
    pub verses: Vec<VerseText>,
}

/// The text of one verse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerseText {
    /// The book.
    pub book: BookId,
    /// Chapter number, from 1.
    pub chapter: u16,
    /// Verse number, from 1.
    pub verse: u16,
    /// The verse text, as one line of plain text.
    pub text: String,
}

/// The file formats [`import`] understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Open Scripture Information Standard (XML).
    Osis,
    /// Zefania XML.
    Zefania,
    /// The OpenSong Bible format (XML).
    OpenSong,
    /// Unified Scripture XML, usually one file per book.
    Usx,
    /// Unified Standard Format Markers, usually one file per book.
    Usfm,
}

/// Why a Bible could not be imported.
#[derive(Debug, Error)]
pub enum ImportError {
    /// The file is not in any format we know.
    #[error(
        "this file is not a Bible in a format SimpleWorship can read (OSIS, Zefania, OpenSong, USX or USFM)"
    )]
    UnknownFormat,
    /// The file looked right but contained no verses we could read.
    #[error("no verses were found in this file")]
    NoVerses,
    /// The XML is broken.
    #[error("the file is damaged: {reason}")]
    Malformed {
        /// What the XML parser reported.
        reason: String,
    },
    /// The zip archive could not be read.
    #[error("the zip file could not be read: {0}")]
    Zip(String),
}

/// One file to import: its name (used to guess the format) and contents.
struct SourceFile {
    name: String,
    text: String,
}

/// Imports a Bible file from disk: any format [`import`] reads, or an
/// EasyWorship `.ewb` database.
pub fn import_file(path: &std::path::Path) -> Result<BibleData, ImportError> {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = std::fs::read(path).map_err(|error| ImportError::Malformed {
        reason: error.to_string(),
    })?;

    if bytes.starts_with(b"SQLite format 3\0") {
        let mut bible = ewb::read(path)?;
        finish(&mut bible);
        if bible.verses.is_empty() {
            return Err(ImportError::NoVerses);
        }
        if bible.abbreviation.is_empty() {
            bible.abbreviation = suggest_abbreviation(&bible.name);
        }
        return Ok(bible);
    }
    import(&name, &bytes)
}

/// Imports a Bible from the bytes of a file or a zip archive of files.
pub fn import(file_name: &str, bytes: &[u8]) -> Result<BibleData, ImportError> {
    let files = if bytes.starts_with(b"PK\x03\x04") {
        unzip(bytes)?
    } else {
        vec![SourceFile {
            name: file_name.to_owned(),
            text: decode(bytes),
        }]
    };

    let format = files
        .iter()
        .find_map(|file| detect(&file.name, &file.text))
        .ok_or(ImportError::UnknownFormat)?;
    let inputs: Vec<&str> = files
        .iter()
        .filter(|file| detect(&file.name, &file.text) == Some(format))
        .map(|file| file.text.as_str())
        .collect();

    let mut bible = match format {
        Format::Osis => osis::read(inputs[0])?,
        Format::Zefania => zefania::read(inputs[0])?,
        Format::OpenSong => opensong::read(inputs[0])?,
        Format::Usx => usx::read(&inputs)?,
        Format::Usfm => usfm::read(&inputs)?,
    };

    finish(&mut bible);
    if bible.verses.is_empty() {
        return Err(ImportError::NoVerses);
    }
    if bible.name.is_empty() {
        bible.name = file_stem(file_name);
    }
    if bible.abbreviation.is_empty() {
        bible.abbreviation = suggest_abbreviation(&bible.name);
    }
    Ok(bible)
}

/// Works out a file's format from the start of its contents.
pub fn detect(file_name: &str, text: &str) -> Option<Format> {
    let head: String = text.chars().take(4096).collect::<String>().to_lowercase();

    if head.contains("<osis") {
        Some(Format::Osis)
    } else if head.contains("<xmlbible") {
        Some(Format::Zefania)
    } else if head.contains("<usx") {
        Some(Format::Usx)
    } else if head.contains("<bible") && head.contains("<b ") {
        Some(Format::OpenSong)
    } else if head.contains("\\id ") || head.contains("\\c ") {
        Some(Format::Usfm)
    } else {
        let lower = file_name.to_lowercase();
        (lower.ends_with(".usfm") || lower.ends_with(".sfm")).then_some(Format::Usfm)
    }
}

/// Reads every file in a zip, skipping folders and macOS metadata.
fn unzip(bytes: &[u8]) -> Result<Vec<SourceFile>, ImportError> {
    let zip_error = |error: zip::result::ZipError| ImportError::Zip(error.to_string());
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_error)?;

    let mut files = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let name = entry.name().map_err(zip_error)?.into_owned();
        if entry.is_dir() || name.starts_with("__MACOSX") || name.contains("/.") {
            continue;
        }
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .map_err(|error| ImportError::Zip(error.to_string()))?;
        files.push(SourceFile {
            name,
            text: decode(&contents),
        });
    }
    // Book files are named "01-GEN.usfm" and so on; reading them in name
    // order keeps books in order even before the final sort.
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Decodes text as UTF-8 (dropping a byte-order mark). Invalid bytes become
/// the replacement character instead of failing the whole import.
fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// Tidies the verses every importer produced: cleans whitespace, drops
/// empty verses, sorts, and keeps only the first copy of a duplicate.
fn finish(bible: &mut BibleData) {
    for verse in &mut bible.verses {
        verse.text = clean_text(&verse.text);
    }
    bible.verses.retain(|verse| !verse.text.is_empty());
    bible
        .verses
        .sort_by_key(|verse| (verse.book, verse.chapter, verse.verse));
    bible
        .verses
        .dedup_by_key(|verse| (verse.book, verse.chapter, verse.verse));

    for field in [
        &mut bible.name,
        &mut bible.abbreviation,
        &mut bible.language,
        &mut bible.copyright,
    ] {
        *field = clean_text(field);
    }
}

/// Collapses runs of whitespace (including line breaks) into single spaces,
/// removes the stray space that markup often leaves before punctuation, and
/// drops pilcrows (¶), which some editions of the KJV use to mark paragraphs.
pub(crate) fn clean_text(text: &str) -> String {
    let collapsed = text
        .split_whitespace()
        .filter(|word| *word != "¶")
        .map(|word| word.trim_start_matches('¶'))
        .collect::<Vec<_>>()
        .join(" ");
    let mut cleaned = String::with_capacity(collapsed.len());
    for c in collapsed.chars() {
        if matches!(c, ',' | '.' | ';' | ':' | '!' | '?') && cleaned.ends_with(' ') {
            cleaned.pop();
        }
        cleaned.push(c);
    }
    cleaned
}

/// A short name for a Bible that has none: a short code in brackets if the
/// name has one ("World English Bible (WEB)" → "WEB"), otherwise the
/// initials ("King James Version" → "KJV").
pub fn suggest_abbreviation(name: &str) -> String {
    let bracketed = name
        .rsplit_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(inside, _)| inside.trim())
        .filter(|inside| !inside.is_empty() && inside.len() <= 8 && !inside.contains(' '));
    if let Some(code) = bracketed {
        return code.to_uppercase();
    }

    let initials: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .take(6)
        .collect();
    if initials.is_empty() {
        "BIBLE".to_owned()
    } else {
        initials
    }
}

/// The leading number of a value such as `"16"`, `"16-17"` or `"16a"`.
/// Verse bridges ("16-17") are filed under their first verse.
pub(crate) fn leading_number(text: &str) -> Option<u16> {
    let digits: String = text
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok().filter(|&n| n > 0)
}

fn file_stem(file_name: &str) -> String {
    let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_owned()
}

/// Gathers verse text as an importer walks through a file.
///
/// An importer calls [`start`](Self::start) when a verse begins and
/// [`push`](Self::push) for each piece of its text. A verse ends when the
/// next one starts, or when [`end`](Self::end) is called (for formats that
/// mark verse ends explicitly).
#[derive(Default)]
pub(crate) struct VerseCollector {
    verses: Vec<VerseText>,
    current: Option<VerseText>,
}

impl VerseCollector {
    /// Starts a new verse, finishing the previous one.
    pub fn start(&mut self, book: BookId, chapter: u16, verse: u16) {
        self.end();
        self.current = Some(VerseText {
            book,
            chapter,
            verse,
            text: String::new(),
        });
    }

    /// Adds text to the current verse. Text outside any verse is ignored.
    pub fn push(&mut self, text: &str) {
        if let Some(current) = &mut self.current {
            current.text.push_str(text);
        }
    }

    /// Whether a verse is open.
    pub fn in_verse(&self) -> bool {
        self.current.is_some()
    }

    /// Finishes the current verse, if any.
    pub fn end(&mut self) {
        if let Some(verse) = self.current.take() {
            self.verses.push(verse);
        }
    }

    /// All verses collected.
    pub fn into_verses(mut self) -> Vec<VerseText> {
        self.end();
        self.verses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_whitespace_and_stray_spaces() {
        assert_eq!(
            clean_text("  In the\n beginning  God ,\tcreated . "),
            "In the beginning God, created."
        );
        assert_eq!(clean_text("¶ For God so loved"), "For God so loved");
    }

    #[test]
    fn detects_formats_from_their_contents() {
        assert_eq!(
            detect("a.xml", "<?xml?><osis xmlns='x'>"),
            Some(Format::Osis)
        );
        assert_eq!(
            detect("a.xml", "<XMLBIBLE biblename='x'>"),
            Some(Format::Zefania)
        );
        assert_eq!(
            detect("a.xml", "<bible><b n='Genesis'>"),
            Some(Format::OpenSong)
        );
        assert_eq!(detect("a.usx", "<usx version='3.0'>"), Some(Format::Usx));
        assert_eq!(detect("GEN.SFM", "\\id GEN\n\\c 1"), Some(Format::Usfm));
        assert_eq!(detect("notes.txt", "hello"), None);
    }

    #[test]
    fn suggests_abbreviations() {
        assert_eq!(suggest_abbreviation("World English Bible (WEB)"), "WEB");
        assert_eq!(suggest_abbreviation("King James Version"), "KJV");
        assert_eq!(suggest_abbreviation(""), "BIBLE");
    }

    #[test]
    fn rejects_unknown_files() {
        assert!(matches!(
            import("x.txt", b"hello"),
            Err(ImportError::UnknownFormat)
        ));
    }
}
