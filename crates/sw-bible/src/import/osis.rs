//! OSIS (Open Scripture Information Standard) XML.
//!
//! Verses come in two styles, and both are handled:
//!
//! ```xml
//! <!-- Container: the text is inside the element. -->
//! <verse osisID="Gen.1.1">In the beginning…</verse>
//!
//! <!-- Milestones: empty start and end markers around the text. -->
//! <verse sID="Gen.1.1" osisID="Gen.1.1"/>In the beginning…<verse eID="Gen.1.1"/>
//! ```
//!
//! Notes, headings and variant readings are skipped. Word-level markup
//! (`<w lemma="strong:H7225">beginning</w>`) keeps its text.

use super::xml::{XmlEvent, attribute, for_each_event};
use super::{BibleData, ImportError, VerseCollector, leading_number};
use crate::books;

/// Elements whose text is not part of the verse.
const SKIPPED: &[&str] = &["note", "title", "rdg", "speaker", "catchword"];

/// Reads an OSIS document.
pub fn read(xml: &str) -> Result<BibleData, ImportError> {
    let mut bible = BibleData::default();
    let mut verses = VerseCollector::default();
    // Open, non-empty elements; used to know when we are inside a skipped one.
    let mut open: Vec<String> = Vec::new();
    // Text of the <work> header element currently open, for metadata.
    let mut header_field: Option<String> = None;

    for_each_event(xml, |event| {
        match event {
            XmlEvent::Start {
                name,
                attributes,
                empty,
            } => {
                if name == "verse" {
                    if attributes.contains_key("eid") {
                        verses.end();
                    } else if let Some(id) =
                        attribute(&attributes, "osisid").or_else(|| attribute(&attributes, "sid"))
                        && let Some((book, chapter, verse)) = parse_osis_id(id)
                    {
                        verses.start(book, chapter, verse);
                    }
                } else if name == "lb" || name == "l" {
                    verses.push(" ");
                }

                if matches!(
                    name.as_str(),
                    "title" | "rights" | "language" | "identifier"
                ) && open.last().is_some_and(|parent| parent == "work")
                {
                    header_field = Some(name.clone());
                }
                if !empty {
                    open.push(name);
                }
            }
            XmlEvent::End { name } => {
                open.pop();
                if name == "verse" {
                    // The end of a container verse.
                    verses.end();
                }
                header_field = None;
            }
            XmlEvent::Text(text) => {
                if let Some(field) = &header_field {
                    store_header(&mut bible, field, &text);
                } else if !open.iter().any(|name| SKIPPED.contains(&name.as_str())) {
                    verses.push(&text);
                }
            }
        }
        Ok(())
    })?;

    bible.verses = verses.into_verses();
    Ok(bible)
}

/// Splits `"Gen.1.1"` into the book, chapter and verse. Combined verses
/// (`"Gen.1.1 Gen.1.2"`) are filed under the first one.
fn parse_osis_id(id: &str) -> Option<(books::BookId, u16, u16)> {
    let first = id.split_whitespace().next()?;
    // Some files prefix the work: "KJV:Gen.1.1".
    let reference = first.rsplit(':').next()?;
    let mut parts = reference.split('.');
    let book = books::by_osis(parts.next()?)?.id;
    let chapter = leading_number(parts.next()?)?;
    let verse = leading_number(parts.next()?)?;
    Some((book, chapter, verse))
}

fn store_header(bible: &mut BibleData, field: &str, text: &str) {
    let target = match field {
        "title" => &mut bible.name,
        "rights" => &mut bible.copyright,
        "language" => &mut bible.language,
        "identifier" => &mut bible.abbreviation,
        _ => return,
    };
    target.push_str(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ids() {
        assert_eq!(
            parse_osis_id("Gen.1.1").map(|(b, c, v)| (b.0, c, v)),
            Some((1, 1, 1))
        );
        assert_eq!(
            parse_osis_id("KJV:John.3.16").map(|(b, c, v)| (b.0, c, v)),
            Some((43, 3, 16))
        );
        assert_eq!(
            parse_osis_id("Ps.23.1 Ps.23.2").map(|(_, c, v)| (c, v)),
            Some((23, 1))
        );
        assert_eq!(parse_osis_id("Nope.1.1"), None);
    }
}
