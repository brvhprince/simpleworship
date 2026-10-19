//! Parsing Bible references typed by an operator.
//!
//! ```
//! use sw_bible::reference::parse;
//!
//! let passage = parse("jn 3:16-18").unwrap().passage;
//! assert_eq!(passage.to_string(), "John 3:16–18");
//! ```
//!
//! The parser is forgiving, because operators type fast:
//!
//! | Typed              | Means                         |
//! | ------------------ | ----------------------------- |
//! | `jn 3:16`          | John 3:16                     |
//! | `John 3.16-18`     | John 3:16–18                  |
//! | `1co13:4-7`        | 1 Corinthians 13:4–7          |
//! | `ps 23`            | the whole of Psalm 23         |
//! | `gen 1:1-2:3`      | across a chapter boundary     |
//! | `rom 8:28, 31-39`  | several ranges in one chapter |
//! | `isa 53; 55:1`     | `;` starts a new chapter      |
//! | `john 3:`          | John 3 (still typing)         |
//!
//! A book name may be partial (`"joh"`). When it could be several books,
//! the first in canonical order is chosen and the others are returned as
//! [`ParsedReference::alternatives`], so the UI can offer them.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::books::{BOOKS, BookId, normalize_name};

/// A position in a chapter. `verse: None` means "the whole chapter": the
/// first verse when it starts a range, the last verse when it ends one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bible.ts")]
pub struct VersePoint {
    /// Chapter number, from 1.
    pub chapter: u16,
    /// Verse number, from 1, or `None` for the whole chapter.
    pub verse: Option<u16>,
}

/// A continuous run of verses, from `start` to `end` inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bible.ts")]
pub struct VerseRange {
    /// The first verse (or chapter).
    pub start: VersePoint,
    /// The last verse (or chapter).
    pub end: VersePoint,
}

/// A passage in one book: one or more verse ranges, e.g. Romans 8:28, 31–39.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bible.ts")]
pub struct Passage {
    /// The book.
    pub book: BookId,
    /// The ranges, in the order they were typed.
    pub ranges: Vec<VerseRange>,
}

/// The result of parsing: the passage, plus other books a partial name
/// could have meant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedReference {
    /// The passage.
    pub passage: Passage,
    /// Other books the typed name could also mean, e.g. for `"ju"`
    /// (parsed as Judges) this contains Jude.
    pub alternatives: Vec<BookId>,
}

/// Why a reference could not be understood.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ReferenceError {
    /// Nothing was typed.
    #[error("type a reference, e.g. “John 3:16”")]
    Empty,
    /// The book name matches no book.
    #[error("there is no book called “{0}”")]
    UnknownBook(String),
    /// Something after the book name is not a chapter or verse.
    #[error("“{0}” is not a chapter or verse")]
    Unexpected(String),
    /// Chapter and verse numbers start at 1.
    #[error("chapters and verses start at 1")]
    Zero,
    /// A range that ends before it starts, e.g. `3:18-16`.
    #[error("the range ends before it starts")]
    Backwards,
}

/// Parses a typed reference. See the [module docs](self) for what is accepted.
pub fn parse(input: &str) -> Result<ParsedReference, ReferenceError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ReferenceError::Empty);
    }

    let (book_part, numbers_part) = split_book(input);
    let (book, alternatives) = find_book(book_part)?;
    let ranges = parse_ranges(numbers_part)?;

    Ok(ParsedReference {
        passage: Passage { book, ranges },
        alternatives,
    })
}

/// Splits `"1 cor 13:4"` into `("1 cor", "13:4")`: the book name ends at the
/// first digit that comes after a letter.
fn split_book(input: &str) -> (&str, &str) {
    let mut seen_letter = false;
    for (index, c) in input.char_indices() {
        if c.is_alphabetic() {
            seen_letter = true;
        } else if c.is_ascii_digit() && seen_letter {
            return (input[..index].trim(), input[index..].trim());
        }
    }
    (input, "")
}

/// Finds the book a typed name means: an exact name or abbreviation if
/// there is one, otherwise every book the name is the start of.
fn find_book(typed: &str) -> Result<(BookId, Vec<BookId>), ReferenceError> {
    let key = normalize_name(typed);
    if key.is_empty() {
        return Err(ReferenceError::UnknownBook(typed.to_owned()));
    }

    if let Some(book) = BOOKS.iter().find(|book| book.matches_exactly(&key)) {
        return Ok((book.id, Vec::new()));
    }

    let mut candidates = BOOKS
        .iter()
        .filter(|book| book.matches_prefix(&key))
        .map(|book| book.id);
    let first = candidates
        .next()
        .ok_or_else(|| ReferenceError::UnknownBook(typed.to_owned()))?;
    Ok((first, candidates.collect()))
}

/// Parses the chapter-and-verse part, e.g. `"8:28, 31-39; 9:1"`. An empty
/// string means chapter 1, so a bare book name shows its first chapter.
fn parse_ranges(text: &str) -> Result<Vec<VerseRange>, ReferenceError> {
    let tokens = tokenize(text)?;
    let mut parser = RangeParser {
        tokens: &tokens,
        position: 0,
    };
    let ranges = parser.ranges()?;

    if ranges.is_empty() {
        let chapter = VersePoint {
            chapter: 1,
            verse: None,
        };
        return Ok(vec![VerseRange {
            start: chapter,
            end: chapter,
        }]);
    }
    Ok(ranges)
}

/// The pieces of a chapter-and-verse string.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Token {
    Number(u16),
    /// `:` or `.` between chapter and verse.
    VerseMark,
    /// `-`, `–` or `—`.
    Dash,
    /// `,` (same chapter continues).
    Comma,
    /// `;` (a new chapter follows).
    Semicolon,
}

fn tokenize(text: &str) -> Result<Vec<Token>, ReferenceError> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start, c)) = chars.next() {
        let token = match c {
            '0'..='9' => {
                let mut end = start + 1;
                while let Some(&(index, next)) = chars.peek() {
                    if !next.is_ascii_digit() {
                        break;
                    }
                    end = index + 1;
                    chars.next();
                }
                let number: u16 = text[start..end]
                    .parse()
                    .map_err(|_| ReferenceError::Unexpected(text[start..end].to_owned()))?;
                if number == 0 {
                    return Err(ReferenceError::Zero);
                }
                Token::Number(number)
            }
            ':' | '.' => Token::VerseMark,
            '-' | '–' | '—' => Token::Dash,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            c if c.is_whitespace() => continue,
            other => return Err(ReferenceError::Unexpected(other.to_string())),
        };
        tokens.push(token);
    }
    Ok(tokens)
}

/// A small recursive-descent parser over the tokens.
///
/// ```text
/// ranges  := group ((',' | ';') group)*
/// group   := point ('-' point)?
/// point   := NUMBER (':' NUMBER)?
/// ```
///
/// What a lone number means depends on context: after `,` it is another
/// verse in the current chapter; after `;` (or at the start) a chapter.
struct RangeParser<'a> {
    tokens: &'a [Token],
    position: usize,
}

impl RangeParser<'_> {
    fn ranges(&mut self) -> Result<Vec<VerseRange>, ReferenceError> {
        let mut ranges = Vec::new();
        // The chapter a lone verse number refers to, after "3:16, 18".
        let mut verse_context: Option<u16> = None;

        while self.position < self.tokens.len() {
            let Some(range) = self.group(verse_context)? else {
                break; // Trailing punctuation while typing, e.g. "john 3:16,".
            };
            verse_context = range.end.verse.map(|_| range.end.chapter);
            ranges.push(range);

            match self.peek() {
                Some(Token::Comma) => self.position += 1,
                Some(Token::Semicolon) => {
                    self.position += 1;
                    verse_context = None;
                }
                None => break,
                Some(token) => return Err(unexpected(token)),
            }
        }
        Ok(ranges)
    }

    /// One `start` or `start-end` group. Returns `None` if the tokens run out
    /// before a number.
    fn group(&mut self, verse_context: Option<u16>) -> Result<Option<VerseRange>, ReferenceError> {
        let Some(start) = self.point(verse_context)? else {
            return Ok(None);
        };

        let mut end = start;
        if self.peek() == Some(Token::Dash) {
            self.position += 1;
            // After "3:16-" a lone number is a verse in chapter 3; after
            // "3-" it is a chapter.
            let context = start.verse.map(|_| start.chapter);
            if let Some(point) = self.point(context)? {
                end = point;
            }
        }

        if (end.chapter, end.verse.unwrap_or(u16::MAX)) < (start.chapter, start.verse.unwrap_or(0))
        {
            return Err(ReferenceError::Backwards);
        }
        Ok(Some(VerseRange { start, end }))
    }

    /// `NUMBER` or `NUMBER:NUMBER`. A lone number is a verse in
    /// `verse_context` if there is one, otherwise a chapter.
    fn point(&mut self, verse_context: Option<u16>) -> Result<Option<VersePoint>, ReferenceError> {
        let Some(Token::Number(first)) = self.peek() else {
            return match self.peek() {
                None => Ok(None),
                Some(token) => Err(unexpected(token)),
            };
        };
        self.position += 1;

        if self.peek() == Some(Token::VerseMark) {
            self.position += 1;
            let verse = match self.peek() {
                Some(Token::Number(verse)) => {
                    self.position += 1;
                    Some(verse)
                }
                // "john 3:" while typing: the whole chapter for now.
                _ => None,
            };
            return Ok(Some(VersePoint {
                chapter: first,
                verse,
            }));
        }

        Ok(Some(match verse_context {
            Some(chapter) => VersePoint {
                chapter,
                verse: Some(first),
            },
            None => VersePoint {
                chapter: first,
                verse: None,
            },
        }))
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.position).copied()
    }
}

fn unexpected(token: Token) -> ReferenceError {
    let text = match token {
        Token::Number(n) => n.to_string(),
        Token::VerseMark => ":".into(),
        Token::Dash => "-".into(),
        Token::Comma => ",".into(),
        Token::Semicolon => ";".into(),
    };
    ReferenceError::Unexpected(text)
}

impl fmt::Display for Passage {
    /// Formats the passage the way it is printed in books:
    /// "John 3:16–18", "Romans 8:28, 31–39", "Psalm 23".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut name = self.book.name();
        // "Psalm 23", but "Psalms 1–3".
        let single_chapter =
            self.ranges.len() == 1 && self.ranges[0].start.chapter == self.ranges[0].end.chapter;
        if name == "Psalms" && single_chapter {
            name = "Psalm".to_owned();
        }
        write!(f, "{name} ")?;

        let mut previous_chapter: Option<u16> = None;
        for (index, range) in self.ranges.iter().enumerate() {
            // Within the same chapter, later ranges leave out the chapter.
            let same_chapter =
                previous_chapter == Some(range.start.chapter) && range.start.verse.is_some();
            if index > 0 {
                f.write_str(if same_chapter { ", " } else { "; " })?;
            }
            write_range(f, range, same_chapter)?;
            previous_chapter = Some(range.end.chapter);
        }
        Ok(())
    }
}

fn write_range(f: &mut fmt::Formatter<'_>, range: &VerseRange, omit_chapter: bool) -> fmt::Result {
    let VerseRange { start, end } = range;

    match start.verse {
        Some(verse) if omit_chapter => write!(f, "{verse}")?,
        Some(verse) => write!(f, "{}:{verse}", start.chapter)?,
        None => write!(f, "{}", start.chapter)?,
    }

    if start == end {
        return Ok(());
    }
    match (start.verse, end.verse) {
        (Some(_), Some(verse)) if start.chapter == end.chapter => write!(f, "–{verse}"),
        (_, Some(verse)) => write!(f, "–{}:{verse}", end.chapter),
        (_, None) => write!(f, "–{}", end.chapter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses and formats, so the expected value is easy to read.
    fn canonical(input: &str) -> String {
        parse(input).unwrap().passage.to_string()
    }

    #[test]
    fn accepts_the_common_spellings() {
        let cases = [
            ("jn 3:16", "John 3:16"),
            ("John 3.16-18", "John 3:16–18"),
            ("john 3:16–18", "John 3:16–18"),
            ("1co13:4-7", "1 Corinthians 13:4–7"),
            ("1 cor 13", "1 Corinthians 13"),
            ("I Cor 13:1", "1 Corinthians 13:1"),
            ("First John 4:8", "1 John 4:8"),
            ("ps 23", "Psalm 23"),
            ("ps23", "Psalm 23"),
            ("Psalms 1-3", "Psalms 1–3"),
            ("gen 1:1-2:3", "Genesis 1:1–2:3"),
            ("rom 8:28, 31-39", "Romans 8:28, 31–39"),
            ("isa 53; 55:1", "Isaiah 53; 55:1"),
            ("song of songs 2:1", "Song of Songs 2:1"),
            ("Song of Solomon 2:1", "Song of Songs 2:1"),
            ("rev 22:20", "Revelation 22:20"),
            ("  Matt   5 : 3 - 12 ", "Matthew 5:3–12"),
        ];
        for (input, expected) in cases {
            assert_eq!(canonical(input), expected, "input {input:?}");
        }
    }

    #[test]
    fn half_typed_references_still_preview() {
        assert_eq!(canonical("john"), "John 1");
        assert_eq!(canonical("john 3:"), "John 3");
        assert_eq!(canonical("john 3:16-"), "John 3:16");
        assert_eq!(canonical("john 3:16,"), "John 3:16");
    }

    #[test]
    fn partial_book_names_choose_the_first_book_and_offer_others() {
        let parsed = parse("ju 1").unwrap();
        assert_eq!(parsed.passage.book.name(), "Judges");
        let alternatives: Vec<_> = parsed.alternatives.iter().map(|b| b.name()).collect();
        assert!(
            alternatives.contains(&"Jude".to_owned()),
            "{alternatives:?}"
        );

        // An exact abbreviation is never ambiguous.
        assert!(parse("jn 1").unwrap().alternatives.is_empty());
        // "phi" could be Philippians or Philemon; Philippians comes first.
        assert_eq!(
            parse("phi 4:13").unwrap().passage.book.name(),
            "Philippians"
        );
    }

    #[test]
    fn whole_chapters_and_verse_ranges_have_the_right_shape() {
        let range = parse("ps 23").unwrap().passage.ranges[0];
        assert_eq!(
            range.start,
            VersePoint {
                chapter: 23,
                verse: None
            }
        );
        assert_eq!(range.end, range.start);

        let ranges = parse("rom 8:28, 31-39").unwrap().passage.ranges;
        assert_eq!(ranges.len(), 2);
        assert_eq!(
            ranges[1].start,
            VersePoint {
                chapter: 8,
                verse: Some(31)
            }
        );
        assert_eq!(
            ranges[1].end,
            VersePoint {
                chapter: 8,
                verse: Some(39)
            }
        );
    }

    #[test]
    fn reports_what_is_wrong() {
        assert_eq!(parse("  "), Err(ReferenceError::Empty));
        assert_eq!(
            parse("hezekiah 3:16"),
            Err(ReferenceError::UnknownBook("hezekiah".into()))
        );
        assert_eq!(parse("john 3:18-16"), Err(ReferenceError::Backwards));
        assert_eq!(parse("john 0:1"), Err(ReferenceError::Zero));
        assert_eq!(
            parse("john 3:16?"),
            Err(ReferenceError::Unexpected("?".into()))
        );
    }
}
