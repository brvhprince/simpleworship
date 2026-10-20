//! Finding Bible references in what a preacher says, from a transcript of
//! the sermon (see the desktop app's Listen).
//!
//! ```
//! use sw_bible::spoken::Spotter;
//!
//! let mut spotter = Spotter::default();
//! let heard = spotter.hear("Turn with me to John chapter three, verse sixteen.");
//! assert_eq!(heard[0].passage.to_string(), "John 3:16");
//! ```
//!
//! People say references in many ways, and speech-to-text writes them in
//! many more:
//!
//! | Said (or transcribed)                       | Means                    |
//! | ------------------------------------------- | ------------------------ |
//! | `John 3:16`, `John 3.16`, `John 3, 16`      | John 3:16                |
//! | `John chapter three verse sixteen`          | John 3:16                |
//! | `First Corinthians thirteen four to seven`  | 1 Corinthians 13:4–7     |
//! | `Romans 8 verses 28 through 30`             | Romans 8:28–30           |
//! | `Psalm one hundred and nineteen`            | Psalm 119                |
//! | `verse seventeen` (after John 3:16)         | John 3:17                |
//! | `the next verse` (after John 3:16)          | John 3:17                |
//! | `chapter four verse two` (after John 3:16)  | John 4:2                 |
//!
//! Only whole book names count ("Isaiah", not "is"), and a book needs a
//! chapter after it, so ordinary speech ("I am", "a mark", "my job") is not
//! taken for a reference. The last reference heard is remembered, so a
//! preacher can go on with "verse seventeen".

use crate::books::{BOOKS, BookId, normalize_name};
use crate::reference::{Passage, VersePoint, VerseRange};

/// A reference found in what was said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heard {
    /// What it refers to.
    pub passage: Passage,
    /// The words it was found in, as transcribed ("John chapter three,
    /// verse sixteen"), to show the operator why it was suggested.
    pub said: String,
}

/// Finds references in a sermon's transcript, a piece at a time, and
/// remembers the last one so "verse seventeen" can follow it.
#[derive(Clone, Debug, Default)]
pub struct Spotter {
    last: Option<Last>,
}

/// The last reference heard: its book, chapter and (last) verse.
#[derive(Clone, Copy, Debug)]
struct Last {
    book: BookId,
    chapter: u16,
    verse: Option<u16>,
}

/// The highest chapter or verse number anyone would say (Psalm 119 has 176
/// verses); anything above is not a reference.
const HIGHEST: u16 = 176;

/// Book names people say that are not the book's own name, already
/// normalized. (The short abbreviations in the book table are for typing,
/// and too often ordinary words when spoken.)
const SPOKEN_NAMES: &[(&str, u8)] = &[
    ("psalm", 19),
    ("songofsolomon", 22),
    ("canticles", 22),
    ("revelations", 66),
    ("apocalypse", 66),
];

impl Spotter {
    /// The references in `text`, a piece of the transcript, in the order
    /// they were said.
    pub fn hear(&mut self, text: &str) -> Vec<Heard> {
        let tokens = merge_numbers(tokenize(text));
        let mut heard = Vec::new();
        let mut index = 0;
        while index < tokens.len() {
            match self.reference_at(&tokens, index) {
                Some((passage, last, end)) => {
                    let said = text[tokens[index].start..tokens[end - 1].end].to_owned();
                    self.last = Some(last);
                    heard.push(Heard { passage, said });
                    index = end;
                }
                None => index += 1,
            }
        }
        heard
    }

    /// Forgets the last reference (a new sermon).
    pub fn forget(&mut self) {
        self.last = None;
    }

    /// A reference starting at `tokens[index]`: the passage, what to
    /// remember, and the index just after it.
    fn reference_at(&self, tokens: &[Token], index: usize) -> Option<(Passage, Last, usize)> {
        if let Some((book, after)) = book_at(tokens, index) {
            let mut at = skip_words(tokens, after, &["chapter"]);
            let chapter = number(tokens, at)?;
            at += 1;
            let (verses, end) = verses_after(tokens, at, true);
            return Some(passage(book, chapter, verses, end));
        }

        // Going on from the last reference.
        let last = self.last?;
        let word = |at: usize| tokens.get(at).and_then(Token::word);
        match word(index)? {
            // "chapter four (verse two)": the same book.
            "chapter" => {
                let chapter = number(tokens, index + 1)?;
                let (verses, end) = verses_after(tokens, index + 2, false);
                Some(passage(last.book, chapter, verses, end))
            }
            // "verse seventeen", "verses eighteen to twenty".
            "verse" | "verses" => {
                let (verses, end) = verses_after(tokens, index, false);
                Some(passage(last.book, last.chapter, Some(verses?), end))
            }
            // "the next verse".
            "next" if word(index + 1) == Some("verse") => {
                let verse = last.verse? + 1;
                Some(passage(
                    last.book,
                    last.chapter,
                    Some(vec![(verse, verse)]),
                    index + 2,
                ))
            }
            _ => None,
        }
    }
}

/// Builds the passage, and what to remember of it.
fn passage(
    book: BookId,
    chapter: u16,
    verses: Option<Vec<(u16, u16)>>,
    end: usize,
) -> (Passage, Last, usize) {
    let point = |verse| VersePoint { chapter, verse };
    let ranges = match &verses {
        Some(verses) => verses
            .iter()
            .map(|&(start, end)| VerseRange {
                start: point(Some(start)),
                end: point(Some(end)),
            })
            .collect(),
        None => vec![VerseRange {
            start: point(None),
            end: point(None),
        }],
    };
    let last = Last {
        book,
        chapter,
        verse: verses.and_then(|verses| verses.last().map(|&(_, end)| end)),
    };
    (Passage { book, ranges }, last, end)
}

/// The verses after a chapter: "verse 16", ":16", "16" (when `bare` is
/// allowed), then "to 18", then "and 20". `None` if there are none (the
/// whole chapter); the index is just after what was used.
fn verses_after(tokens: &[Token], at: usize, bare: bool) -> (Option<Vec<(u16, u16)>>, usize) {
    let mut at = skip_words(tokens, at, &["from", "at"]);
    let marked = matches!(tokens.get(at).map(|t| &t.kind), Some(Kind::Colon))
        || matches!(
            tokens.get(at).and_then(Token::word),
            Some("verse" | "verses" | "vs")
        );
    if marked {
        at += 1;
    } else if !bare {
        return (None, at);
    }
    let Some(first) = number(tokens, at) else {
        return (None, if marked { at - 1 } else { at });
    };
    at += 1;

    let mut verses = vec![(first, first)];
    loop {
        let joiner = tokens.get(at);
        let range = matches!(joiner.map(|t| &t.kind), Some(Kind::Dash))
            || matches!(
                joiner.and_then(Token::word),
                Some("to" | "through" | "thru" | "till" | "until")
            );
        let and = joiner.and_then(Token::word) == Some("and");
        if !range && !and {
            break;
        }
        // "to verse 18" says "verse" again.
        let next = skip_words(tokens, at + 1, &["verse", "verses"]);
        let Some(verse) = number(tokens, next) else {
            break;
        };
        let last = verses.last_mut().expect("there is a first verse");
        if range && verse >= last.1 {
            last.1 = verse;
        } else if and {
            verses.push((verse, verse));
        } else {
            break;
        }
        at = next + 1;
    }
    (Some(verses), at)
}

/// The book named at `tokens[index]` (one to four words, e.g. "Song of
/// Songs", "First Corinthians", "1 John"), and the index just after it.
fn book_at(tokens: &[Token], index: usize) -> Option<(BookId, usize)> {
    let first = tokens.get(index)?;
    // A book name starts with a word, or a number of a numbered book.
    if !matches!(first.kind, Kind::Word(_) | Kind::Number(1..=4)) {
        return None;
    }
    (1..=4).rev().find_map(|length| {
        let words = tokens.get(index..index + length)?;
        let mut name = String::new();
        for token in words {
            match &token.kind {
                Kind::Word(word) => name.push_str(word),
                Kind::Number(number) if name.is_empty() => name.push_str(&number.to_string()),
                _ => return None,
            }
            name.push(' ');
        }
        let key = normalize_name(&name);
        let book = BOOKS
            .iter()
            .find(|book| normalize_name(book.name) == key)
            .map(|book| book.id)
            .or_else(|| {
                SPOKEN_NAMES
                    .iter()
                    .find(|(spoken, _)| *spoken == key)
                    .map(|&(_, id)| BookId(id))
            })?;
        Some((book, index + length))
    })
}

/// The chapter or verse number at `tokens[at]`, if it is a sensible one.
fn number(tokens: &[Token], at: usize) -> Option<u16> {
    match tokens.get(at)?.kind {
        Kind::Number(number) if (1..=HIGHEST).contains(&number) => Some(number),
        _ => None,
    }
}

/// Skips any of `words` at `tokens[at]` (e.g. "chapter" before a number).
fn skip_words(tokens: &[Token], mut at: usize, words: &[&str]) -> usize {
    while tokens
        .get(at)
        .and_then(Token::word)
        .is_some_and(|word| words.contains(&word))
    {
        at += 1;
    }
    at
}

/// A word, number or mark in the transcript, with where it is in the text.
#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: Kind,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    /// A word, in lower case.
    Word(String),
    Number(u16),
    /// `:` (or `.`) between chapter and verse.
    Colon,
    /// `-`, `–` or `—`.
    Dash,
}

impl Token {
    fn word(&self) -> Option<&str> {
        match &self.kind {
            Kind::Word(word) => Some(word),
            _ => None,
        }
    }
}

/// Splits a transcript into words, numbers and marks. Other punctuation
/// separates words and is otherwise ignored ("John 3, 16" is "John 3 16").
fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        let mut end = start + c.len_utf8();
        let kind = if c.is_ascii_digit() {
            while let Some(&(index, next)) = chars.peek() {
                if !next.is_ascii_digit() {
                    break;
                }
                end = index + next.len_utf8();
                chars.next();
            }
            // Too long to be a chapter or verse: not one.
            Kind::Number(text[start..end].parse().unwrap_or(u16::MAX))
        } else if c.is_alphabetic() {
            while let Some(&(index, next)) = chars.peek() {
                if !(next.is_alphabetic() || next == '\'' || next == '’') {
                    break;
                }
                end = index + next.len_utf8();
                chars.next();
            }
            Kind::Word(text[start..end].to_lowercase())
        } else if c == ':' {
            Kind::Colon
        } else if c == '.' {
            // "3.16" is chapter and verse; a full stop is not.
            let after_number = matches!(tokens.last(), Some(Token { kind: Kind::Number(_), end: e, .. }) if *e == start);
            let before_digit = chars.peek().is_some_and(|&(_, next)| next.is_ascii_digit());
            if !(after_number && before_digit) {
                continue;
            }
            Kind::Colon
        } else if matches!(c, '-' | '–' | '—') {
            Kind::Dash
        } else {
            continue;
        };
        tokens.push(Token { kind, start, end });
    }
    tokens
}

/// Turns numbers said in words into numbers: "sixteen" → 16, "twenty
/// three" and "twenty-three" → 23, "one hundred and nineteen" → 119.
fn merge_numbers(tokens: Vec<Token>) -> Vec<Token> {
    let mut merged = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        match number_in_words(&tokens[index..]) {
            Some((value, used)) => {
                merged.push(Token {
                    kind: Kind::Number(value),
                    start: tokens[index].start,
                    end: tokens[index + used - 1].end,
                });
                index += used;
            }
            None => {
                merged.push(tokens[index].clone());
                index += 1;
            }
        }
    }
    merged
}

/// A number said in words at the start of `tokens`, and how many tokens
/// it took.
fn number_in_words(tokens: &[Token]) -> Option<(u16, usize)> {
    let word = |at: usize| tokens.get(at).and_then(Token::word);
    let mut value = 0;
    let mut at = 0;

    // "one hundred", "a hundred", then perhaps "and".
    if let (Some(first), Some("hundred")) = (word(0), word(1))
        && let Some(hundreds) = units(first).or((first == "a").then_some(1))
    {
        value = hundreds * 100;
        at = 2;
        if word(at) == Some("and") && word(at + 1).is_some_and(|w| tens(w).or(units(w)).is_some()) {
            at += 1;
        }
    }
    if let Some(ten) = word(at).and_then(tens) {
        value += ten;
        at += 1;
        // "twenty three" or "twenty-three".
        let unit_at = if matches!(tokens.get(at).map(|t| &t.kind), Some(Kind::Dash)) {
            at + 1
        } else {
            at
        };
        if let Some(unit) = word(unit_at).and_then(units).filter(|&u| u < 10) {
            value += unit;
            at = unit_at + 1;
        }
    } else if let Some(unit) = word(at).and_then(units) {
        value += unit;
        at += 1;
    }
    (at > 0).then_some((value, at))
}

/// One to nineteen.
fn units(word: &str) -> Option<u16> {
    let units = [
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    units
        .iter()
        .position(|&unit| unit == word)
        .map(|index| index as u16 + 1)
}

/// Twenty, thirty … ninety.
fn tens(word: &str) -> Option<u16> {
    let tens = [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    tens.iter()
        .position(|&ten| ten == word)
        .map(|index| (index as u16 + 2) * 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Everything heard in `text`, as display strings.
    fn hear(spotter: &mut Spotter, text: &str) -> Vec<String> {
        spotter
            .hear(text)
            .into_iter()
            .map(|heard| heard.passage.to_string())
            .collect()
    }

    fn heard(text: &str) -> Vec<String> {
        hear(&mut Spotter::default(), text)
    }

    #[test]
    fn written_references() {
        assert_eq!(heard("Turn to John 3:16 with me."), ["John 3:16"]);
        assert_eq!(heard("John 3.16"), ["John 3:16"]);
        assert_eq!(heard("John 3, 16"), ["John 3:16"]);
        assert_eq!(heard("Romans 8:28-30"), ["Romans 8:28–30"]);
        assert_eq!(heard("1 Corinthians 13:4"), ["1 Corinthians 13:4"]);
    }

    #[test]
    fn spoken_references() {
        assert_eq!(heard("John chapter three, verse sixteen"), ["John 3:16"]);
        assert_eq!(
            heard("First Corinthians thirteen four to seven"),
            ["1 Corinthians 13:4–7"]
        );
        assert_eq!(heard("Romans 8 verses 28 through 30"), ["Romans 8:28–30"]);
        assert_eq!(heard("Psalm one hundred and nineteen"), ["Psalm 119"]);
        assert_eq!(heard("Psalm twenty-three"), ["Psalm 23"]);
        assert_eq!(heard("Song of Songs chapter 2"), ["Song of Songs 2"]);
        assert_eq!(
            heard("Isaiah 40 verse 28 and verse 31"),
            ["Isaiah 40:28, 31"]
        );
        assert_eq!(
            heard("Romans 8 from verse 28 to verse 30"),
            ["Romans 8:28–30"]
        );
    }

    #[test]
    fn several_in_one_breath() {
        assert_eq!(
            heard("Compare Genesis 1:1 with John 1:1."),
            ["Genesis 1:1", "John 1:1"]
        );
    }

    #[test]
    fn ordinary_speech_is_not_a_reference() {
        assert!(heard("I am so glad you came to church today").is_empty());
        assert!(heard("Is it well with your soul? Mark my words.").is_empty());
        assert!(heard("Read John, then come back").is_empty());
        assert!(heard("Acts of kindness").is_empty());
        assert!(heard("John 4000").is_empty());
    }

    #[test]
    fn going_on_from_the_last_reference() {
        let mut spotter = Spotter::default();
        assert_eq!(hear(&mut spotter, "John 3:16"), ["John 3:16"]);
        assert_eq!(
            hear(&mut spotter, "and verse seventeen says"),
            ["John 3:17"]
        );
        assert_eq!(hear(&mut spotter, "the next verse"), ["John 3:18"]);
        assert_eq!(hear(&mut spotter, "chapter four verse two"), ["John 4:2"]);
        spotter.forget();
        assert!(hear(&mut spotter, "verse seventeen").is_empty());
    }

    #[test]
    fn a_reference_split_across_two_pieces() {
        // Speech-to-text works in short pieces, so "John 3" and "verse 16"
        // can arrive separately.
        let mut spotter = Spotter::default();
        assert_eq!(hear(&mut spotter, "Let's read John chapter 3"), ["John 3"]);
        assert_eq!(hear(&mut spotter, "verse 16."), ["John 3:16"]);
    }

    #[test]
    fn shows_what_was_said() {
        let found = Spotter::default().hear("Turn with me to John chapter three, verse sixteen.");
        assert_eq!(found[0].said, "John chapter three, verse sixteen");
    }
}
