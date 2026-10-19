//! USFM (Unified Standard Format Markers), one file per book.
//!
//! USFM is plain text with backslash markers:
//!
//! ```text
//! \id GEN World English Bible
//! \h Genesis
//! \c 1
//! \s1 The Creation                         ← heading: skipped
//! \p
//! \v 1 In the beginning, God\f + \fr 1:1 \ft a note\f* created…
//! \v 2 The earth was formless…
//! \q1 a line of poetry, still verse 2
//! ```
//!
//! What is kept is the text after each `\v`, up to the next verse or
//! chapter, minus footnotes (`\f … \f*`), cross-references (`\x … \x*`),
//! word attributes (`\w word|strong="H7225"\w*` keeps "word") and markers.

use super::{BibleData, ImportError, VerseCollector, leading_number};
use crate::books::{self, BookId};

/// Paragraph markers whose whole line is a heading, title or introduction.
/// Compared without their trailing level number ("s1" → "s").
const NON_VERSE_LINES: &[&str] = &[
    "id", "ide", "h", "toc", "toca", "mt", "mte", "ms", "mr", "s", "sr", "r", "d", "sp", "rem",
    "sts", "cl", "cp", "cd", "imt", "is", "ip", "ipi", "im", "io", "iot", "ili", "ie", "iex",
    "usfm", "restore",
];

/// Character-level notes removed together with their contents.
const NOTES: &[(&str, &str)] = &[
    ("\\f ", "\\f*"),
    ("\\fe ", "\\fe*"),
    ("\\ef ", "\\ef*"),
    ("\\x ", "\\x*"),
    ("\\ex ", "\\ex*"),
];

/// Reads the USFM files of a Bible (one per book, in any order).
pub fn read(files: &[&str]) -> Result<BibleData, ImportError> {
    let mut verses = VerseCollector::default();
    let mut name = String::new();

    for text in files {
        read_book(&remove_notes(text), &mut verses, &mut name);
    }
    Ok(BibleData {
        name,
        verses: verses.into_verses(),
        ..BibleData::default()
    })
}

/// Reads one book file. `name` picks up the Bible's name from the first
/// `\id` line that has one ("\id GEN World English Bible").
fn read_book(text: &str, verses: &mut VerseCollector, name: &mut String) {
    let mut book: Option<BookId> = None;
    let mut chapter: Option<u16> = None;

    for line in text.lines() {
        let line = line.trim();
        let (marker, rest) = split_marker(line);

        match marker {
            Some("id") => {
                verses.end();
                let (code, description) = rest.split_once(' ').unwrap_or((rest, ""));
                let known_book = books::by_usfm(code);
                book = known_book.map(|book| book.id);
                if let Some(known_book) = known_book
                    && name.is_empty()
                    && !description.contains(known_book.name)
                {
                    // Skips front matter ("FRT Preface…") and files that
                    // describe the book ("GEN Genesis") instead of the Bible.
                    description.trim().clone_into(name);
                }
                chapter = None;
            }
            Some("c") => {
                verses.end();
                chapter = leading_number(rest);
            }
            Some(marker) if NON_VERSE_LINES.contains(&without_level(marker)) => {}
            _ => read_verse_line(line, book, chapter, verses),
        }
    }
    verses.end();
}

/// Handles a line that may contain verse text and `\v` markers.
fn read_verse_line(
    line: &str,
    book: Option<BookId>,
    chapter: Option<u16>,
    verses: &mut VerseCollector,
) {
    let mut pieces = line.split("\\v ");
    // Text before the first \v continues the current verse (poetry lines,
    // new paragraphs).
    if let Some(continuation) = pieces.next() {
        verses.push(" ");
        verses.push(&plain_text(continuation));
    }
    for piece in pieces {
        let piece = piece.trim_start();
        let (number, text) = piece.split_once(char::is_whitespace).unwrap_or((piece, ""));
        if let (Some(book), Some(chapter), Some(verse)) = (book, chapter, leading_number(number)) {
            verses.start(book, chapter, verse);
            verses.push(&plain_text(text));
        }
    }
}

/// Splits `"\s1 The Creation"` into `(Some("s1"), "The Creation")`.
fn split_marker(line: &str) -> (Option<&str>, &str) {
    let Some(after) = line.strip_prefix('\\') else {
        return (None, line);
    };
    let end = after
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after.len());
    (Some(&after[..end]), after[end..].trim_start())
}

/// Removes footnotes and cross-references, including their text, in a
/// single pass over the book.
fn remove_notes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find('\\') {
        let opened = NOTES
            .iter()
            .find(|(open, _)| rest[start..].starts_with(open));
        let Some((open, close)) = opened else {
            // Some other marker: keep it and carry on after the backslash.
            result.push_str(&rest[..=start]);
            rest = &rest[start + 1..];
            continue;
        };

        result.push_str(&rest[..start]);
        let body = &rest[start + open.len()..];
        rest = match body.find(close) {
            Some(end) => &body[end + close.len()..],
            // An unclosed note runs to the end of the book.
            None => "",
        };
    }
    result.push_str(rest);
    result
}

/// Strips character markers from verse text, keeping the words:
/// `"\wj Love\wj* \w one|strong="G240"\w*"` → `"Love one"`.
fn plain_text(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // A marker such as \wj, \add*, \+nd: drop it, and the single
            // space that follows an opening marker.
            '\\' => {
                let mut closing = false;
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphanumeric() || next == '+' {
                        chars.next();
                    } else {
                        if next == '*' {
                            chars.next();
                            closing = true;
                        }
                        break;
                    }
                }
                if !closing && chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            // Word attributes: "word|strong=…" up to the closing marker.
            '|' => {
                while chars.peek().is_some_and(|&next| next != '\\') {
                    chars.next();
                }
            }
            _ => plain.push(c),
        }
    }
    plain
}

/// `"s1"` → `"s"`, `"toc2"` → `"toc"`.
fn without_level(marker: &str) -> &str {
    marker.trim_end_matches(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markup_but_keeps_words() {
        assert_eq!(
            plain_text(r#"\wj Love\wj* \w one|strong="G240"\w* another"#),
            "Love one another"
        );
        assert_eq!(plain_text(r"the \nd Lord\nd* said"), "the Lord said");
    }

    #[test]
    fn removes_notes_with_their_text() {
        assert_eq!(
            remove_notes(r"God\f + \fr 1:1 \ft a note\f* created\x - \xo 1:1 \xt Jn 1:1\x*."),
            "God created."
        );
    }
}
