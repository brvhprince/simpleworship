//! USX (Unified Scripture XML), one file per book.
//!
//! ```xml
//! <usx version="3.0">
//!   <book code="GEN" style="id">…</book>
//!   <chapter number="1" style="c" sid="GEN 1"/>
//!   <para style="s1">The Creation</para>          <!-- heading: skipped -->
//!   <para style="p">
//!     <verse number="1" style="v" sid="GEN 1:1"/>In the beginning…<verse eid="GEN 1:1"/>
//! ```
//!
//! USX 2 files have no `eid` markers; a verse then runs until the next
//! verse or chapter starts.

use super::xml::{XmlEvent, attribute, for_each_event};
use super::{BibleData, ImportError, VerseCollector, leading_number};
use crate::books::{self, BookId};

/// Paragraph styles that hold headings, titles or introductions rather than
/// verse text. Compared without their trailing level number ("s1" → "s").
const NON_VERSE_PARAGRAPHS: &[&str] = &[
    "h", "toc", "toca", "mt", "mte", "ms", "mr", "s", "sr", "r", "d", "sp", "rem", "ide", "cl",
    "imt", "is", "ip", "ipi", "im", "io", "iot", "ili", "ie", "iex", "cd", "restore",
];

/// Reads the USX files of a Bible (one per book, in any order).
pub fn read(files: &[&str]) -> Result<BibleData, ImportError> {
    let mut verses = VerseCollector::default();
    for xml in files {
        read_book(xml, &mut verses)?;
    }
    Ok(BibleData {
        verses: verses.into_verses(),
        ..BibleData::default()
    })
}

fn read_book(xml: &str, verses: &mut VerseCollector) -> Result<(), ImportError> {
    let mut book: Option<BookId> = None;
    let mut chapter: Option<u16> = None;
    // Open non-empty elements and whether each one hides its text.
    let mut open: Vec<bool> = Vec::new();

    for_each_event(xml, |event| {
        match event {
            XmlEvent::Start {
                name,
                attributes,
                empty,
            } => {
                let mut hides_text = false;
                match name.as_str() {
                    "book" => {
                        book = attribute(&attributes, "code")
                            .and_then(books::by_usfm)
                            .map(|book| book.id);
                        // The <book> element holds a description, not verse text.
                        hides_text = true;
                    }
                    "chapter" if !attributes.contains_key("eid") => {
                        verses.end();
                        chapter = attribute(&attributes, "number").and_then(leading_number);
                    }
                    "verse" if attributes.contains_key("eid") => verses.end(),
                    "verse" => {
                        let verse = attribute(&attributes, "number").and_then(leading_number);
                        if let (Some(book), Some(chapter), Some(verse)) = (book, chapter, verse) {
                            verses.start(book, chapter, verse);
                        }
                    }
                    "note" | "figure" => hides_text = true,
                    "para" => {
                        let style = attribute(&attributes, "style").unwrap_or("");
                        hides_text = NON_VERSE_PARAGRAPHS.contains(&without_level(style));
                    }
                    "optbreak" => verses.push(" "),
                    _ => {}
                }
                if !empty {
                    open.push(hides_text);
                }
            }
            XmlEvent::End { name } => {
                open.pop();
                // A paragraph break between verses is a space.
                if name == "para" {
                    verses.push(" ");
                }
            }
            XmlEvent::Text(text) => {
                if !open.iter().any(|&hides| hides) {
                    verses.push(&text);
                }
            }
        }
        Ok(())
    })?;
    verses.end();
    Ok(())
}

/// `"s1"` → `"s"`, `"toc2"` → `"toc"`.
fn without_level(style: &str) -> &str {
    style.trim_end_matches(|c: char| c.is_ascii_digit())
}
