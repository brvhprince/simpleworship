//! Zefania XML.
//!
//! ```xml
//! <XMLBIBLE biblename="King James Version">
//!   <INFORMATION><title>…</title><rights>…</rights></INFORMATION>
//!   <BIBLEBOOK bnumber="1" bname="Genesis">
//!     <CHAPTER cnumber="1">
//!       <VERS vnumber="1">In the beginning…<NOTE>…</NOTE></VERS>
//! ```
//!
//! Book numbers 1–66 are the usual order. Higher numbers (the deuterocanon)
//! differ between files, so those books are matched by name instead.

use super::xml::{XmlEvent, attribute, for_each_event};
use super::{BibleData, ImportError, VerseCollector, leading_number};
use crate::books::{self, BookId};

/// Reads a Zefania document.
pub fn read(xml: &str) -> Result<BibleData, ImportError> {
    let mut bible = BibleData::default();
    let mut verses = VerseCollector::default();
    let mut book: Option<BookId> = None;
    let mut chapter: Option<u16> = None;
    let mut in_note = 0usize;
    let mut header_field: Option<String> = None;

    for_each_event(xml, |event| {
        match event {
            XmlEvent::Start {
                name,
                attributes,
                empty,
            } => match name.as_str() {
                "xmlbible" => {
                    if let Some(title) = attribute(&attributes, "biblename") {
                        bible.name = title.to_owned();
                    }
                }
                "biblebook" => book = book_from(&attributes),
                "chapter" => chapter = attribute(&attributes, "cnumber").and_then(leading_number),
                "vers" => {
                    let verse = attribute(&attributes, "vnumber").and_then(leading_number);
                    if let (Some(book), Some(chapter), Some(verse)) = (book, chapter, verse) {
                        verses.start(book, chapter, verse);
                    }
                }
                "note" if !empty => in_note += 1,
                "br" => verses.push(" "),
                "title" | "rights" | "language" | "identifier" => header_field = Some(name),
                _ => {}
            },
            XmlEvent::End { name } => match name.as_str() {
                "vers" => verses.end(),
                "note" => in_note = in_note.saturating_sub(1),
                _ => header_field = None,
            },
            XmlEvent::Text(text) => {
                if verses.in_verse() {
                    if in_note == 0 {
                        verses.push(&text);
                    }
                } else if let Some(field) = &header_field {
                    store_header(&mut bible, field, &text);
                }
            }
        }
        Ok(())
    })?;

    bible.verses = verses.into_verses();
    Ok(bible)
}

/// The book of a `<BIBLEBOOK>`: by number for the 66 common books, by name
/// otherwise.
fn book_from(attributes: &std::collections::HashMap<String, String>) -> Option<BookId> {
    let number = attribute(attributes, "bnumber").and_then(leading_number);
    match number {
        Some(n @ 1..=66) => Some(BookId(n as u8)),
        _ => attribute(attributes, "bname")
            .and_then(books::by_name)
            .map(|book| book.id),
    }
}

fn store_header(bible: &mut BibleData, field: &str, text: &str) {
    let target = match field {
        // The biblename attribute is usually better than <title>; keep it.
        "title" if bible.name.is_empty() => &mut bible.name,
        "rights" => &mut bible.copyright,
        "language" => &mut bible.language,
        "identifier" => &mut bible.abbreviation,
        _ => return,
    };
    target.push_str(text);
}
