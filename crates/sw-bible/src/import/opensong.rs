//! The OpenSong Bible format.
//!
//! ```xml
//! <bible>
//!   <b n="Genesis">
//!     <c n="1">
//!       <v n="1">In the beginning…</v>
//! ```
//!
//! Books are named, not numbered. Names are matched against the known book
//! names; a name we do not recognise (another language, say) falls back to
//! its position in the file, which works for the usual 66-book order.

use super::xml::{XmlEvent, attribute, for_each_event};
use super::{BibleData, ImportError, VerseCollector, leading_number};
use crate::books::{self, BookId};

/// Reads an OpenSong Bible.
pub fn read(xml: &str) -> Result<BibleData, ImportError> {
    let mut verses = VerseCollector::default();
    let mut book_count = 0u8;
    let mut book: Option<BookId> = None;
    let mut chapter: Option<u16> = None;

    for_each_event(xml, |event| {
        match event {
            XmlEvent::Start {
                name, attributes, ..
            } => match name.as_str() {
                "b" => {
                    book_count = book_count.saturating_add(1);
                    book = attribute(&attributes, "n")
                        .and_then(books::by_name)
                        .map(|book| book.id)
                        .or_else(|| (book_count <= 66).then_some(BookId(book_count)));
                }
                "c" => chapter = attribute(&attributes, "n").and_then(leading_number),
                "v" => {
                    let verse = attribute(&attributes, "n").and_then(leading_number);
                    if let (Some(book), Some(chapter), Some(verse)) = (book, chapter, verse) {
                        verses.start(book, chapter, verse);
                    }
                }
                _ => {}
            },
            XmlEvent::End { name } if name == "v" => verses.end(),
            XmlEvent::End { .. } => {}
            XmlEvent::Text(text) => verses.push(&text),
        }
        Ok(())
    })?;

    Ok(BibleData {
        verses: verses.into_verses(),
        ..BibleData::default()
    })
}
