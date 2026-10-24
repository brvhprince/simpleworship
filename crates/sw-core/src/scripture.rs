//! Scripture: Bible translations, verses, and turning a passage into slides.
//!
//! A passage can be shown in one translation, or in two side by side. The
//! verses are split into slides by these rules:
//!
//! 1. A slide never mixes chapters.
//! 2. A slide holds at most [`ScriptureOptions::max_verses`] verses.
//! 3. A slide holds at most about [`ScriptureOptions::max_chars`] characters
//!    of text per column, so long verses do not end up tiny on screen.
//! 4. A single verse that is too long on its own (in a one-translation
//!    passage) is split at punctuation into several slides; the later parts
//!    are marked as continued and show no verse number.
//!
//! Translations are matched verse by verse using each one's own numbering.
//! A verse that one translation lacks shows as blank in that column.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use sw_bible::{BookId, Passage, VersePoint, VerseRange};
use ts_rs::TS;

use crate::id::Id;
use crate::slides::Slide;

/// A Bible translation installed in the library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct BibleInfo {
    /// Unique identifier.
    pub id: Id,
    /// Full name, e.g. "King James Version".
    pub name: String,
    /// Short name shown everywhere space is tight, e.g. "KJV".
    pub abbreviation: String,
    /// Language, e.g. "en".
    pub language: String,
    /// Copyright or licence notice.
    pub copyright: String,
    /// How many verses it contains.
    pub verse_count: u32,
    /// Whether it ships with SimpleWorship (and is re-added if deleted on a
    /// fresh install).
    pub bundled: bool,
}

/// One verse of one translation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Verse {
    /// The book.
    pub book: BookId,
    /// Chapter number.
    pub chapter: u16,
    /// Verse number.
    pub verse: u16,
    /// The text.
    pub text: String,
}

/// Which chapters a book has in one Bible, and how many verses each, for
/// the book → chapter → verse picker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct BookOutline {
    /// The book.
    pub book: BookId,
    /// The book's English name.
    pub name: String,
    /// The number of verses in each chapter: `chapters[0]` is chapter 1.
    pub chapters: Vec<u16>,
}

/// The verses of a passage in one translation, with what is needed to label
/// and credit it. Stored inside schedule items, so a saved schedule shows
/// the same text on a computer without that Bible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct TranslationText {
    /// The Bible the text came from.
    pub bible_id: Id,
    /// Its short name, e.g. "KJV".
    pub abbreviation: String,
    /// Its copyright notice.
    pub copyright: String,
    /// The verses, in order.
    pub verses: Vec<Verse>,
}

/// How a passage is split into slides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct ScriptureOptions {
    /// The most verses on one slide.
    pub max_verses: u32,
    /// Roughly the most characters on one slide (per column).
    pub max_chars: u32,
    /// Show verse numbers before each verse, or `None` to do as the theme
    /// says (see [`VerseNumberStyle`](crate::theme::VerseNumberStyle)).
    #[serde(default)]
    pub verse_numbers: Option<bool>,
    /// Automatically flow long scripture onto multiple slides.
    #[serde(default = "default_auto_flow")]
    pub auto_flow: bool,
    /// Minimum font size before breaking slide.
    #[serde(default = "default_min_font_size")]
    pub min_font_size: u32,
}

pub(crate) fn default_auto_flow() -> bool {
    true
}

pub(crate) fn default_min_font_size() -> u32 {
    36
}

impl Default for ScriptureOptions {
    fn default() -> Self {
        Self {
            max_verses: 2,
            max_chars: 300,
            verse_numbers: None,
            auto_flow: true,
            min_font_size: 36,
        }
    }
}

/// The structured content of a scripture slide: one column per translation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct ScriptureSlide {
    /// The columns, left to right.
    pub columns: Vec<ScriptureColumn>,
    /// Whether verse numbers are shown, or `None` to do as the theme says.
    pub verse_numbers: Option<bool>,
}

/// One translation's part of a scripture slide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct ScriptureColumn {
    /// The translation's short name, e.g. "KJV".
    pub translation: String,
    /// The verses on this slide.
    pub verses: Vec<VerseLine>,
}

/// A verse (or part of one) on a slide.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct VerseLine {
    /// The verse number.
    pub number: u16,
    /// The text; empty if this translation lacks the verse.
    pub text: String,
    /// `true` for the second and later parts of a verse split across slides.
    /// Continued parts show no verse number.
    pub continued: bool,
}

/// `"John 3:16–18 (KJV · WEB)"`: the passage plus the translations.
pub fn title(passage: &Passage, translations: &[TranslationText]) -> String {
    format!("{passage} ({})", abbreviations(translations))
}

fn abbreviations(translations: &[TranslationText]) -> String {
    translations
        .iter()
        .map(|t| t.abbreviation.as_str())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The footer for a scripture presentation: the translations' copyright
/// notices, shortened so a long licence text does not fill the screen.
pub fn footer(translations: &[TranslationText]) -> Option<String> {
    const MAX_FOOTER_CHARS: usize = 140;

    let mut notices: Vec<&str> = Vec::new();
    for translation in translations {
        let notice = translation.copyright.trim();
        if !notice.is_empty() && !notices.contains(&notice) {
            notices.push(notice);
        }
    }
    let joined = notices.join(" · ");
    if joined.is_empty() {
        return None;
    }
    Some(if joined.chars().count() > MAX_FOOTER_CHARS {
        let cut: String = joined.chars().take(MAX_FOOTER_CHARS - 1).collect();
        format!("{}…", cut.trim_end())
    } else {
        joined
    })
}

/// Splits a passage into slides. See the [module docs](self) for the rules.
pub fn slides(
    passage: &Passage,
    translations: &[TranslationText],
    options: ScriptureOptions,
) -> Vec<Slide> {
    let columns = translations.len().max(1);
    let per_column_budget = if options.auto_flow {
        let scaled = (36 * options.max_chars as u32) / options.min_font_size.max(12);
        (scaled as usize / columns).max(40)
    } else {
        usize::MAX / 2
    };
    let max_verses = (options.max_verses as usize).max(1);

    // Each translation's text by (chapter, verse), and every verse any of
    // them has, in order.
    let texts: Vec<HashMap<(u16, u16), &str>> = translations
        .iter()
        .map(|t| {
            t.verses
                .iter()
                .map(|v| ((v.chapter, v.verse), v.text.as_str()))
                .collect()
        })
        .collect();
    let keys: BTreeSet<(u16, u16)> = texts.iter().flat_map(|map| map.keys().copied()).collect();

    // The longest translation's length decides how much fits.
    let length = |key: &(u16, u16)| {
        texts
            .iter()
            .map(|map| map.get(key).map_or(0, |text| text.chars().count()))
            .max()
            .unwrap_or(0)
    };

    let mut slides = Vec::new();
    let mut chunk: Vec<(u16, u16)> = Vec::new();
    let mut chunk_length = 0;
    let mut flush = |chunk: &mut Vec<(u16, u16)>, chunk_length: &mut usize| {
        if !chunk.is_empty() {
            slides.extend(build_slides(
                passage.book,
                chunk,
                translations,
                &texts,
                per_column_budget,
                options,
            ));
            chunk.clear();
            *chunk_length = 0;
        }
    };

    for key in keys {
        let verse_length = length(&key);
        let new_chapter = chunk.last().is_some_and(|last| last.0 != key.0);
        let full = chunk.len() >= max_verses || chunk_length + verse_length > per_column_budget;
        if new_chapter || (!chunk.is_empty() && full) {
            flush(&mut chunk, &mut chunk_length);
        }
        chunk.push(key);
        chunk_length += verse_length;
    }
    flush(&mut chunk, &mut chunk_length);
    slides
}

/// Builds the slide(s) for one chunk of verses. A chunk is a single slide,
/// except when auto-flow splits long verse(s).
fn build_slides(
    book: BookId,
    chunk: &[(u16, u16)],
    translations: &[TranslationText],
    texts: &[HashMap<(u16, u16), &str>],
    budget: usize,
    options: ScriptureOptions,
) -> Vec<Slide> {
    let caption = format!(
        "{} ({})",
        chunk_passage(book, chunk),
        abbreviations(translations)
    );

    let splittable = options.auto_flow && chunk.len() == 1;
    if splittable {
        let key = chunk[0];
        let mut all_parts: Vec<Vec<String>> = translations
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let text = texts[i].get(&key).copied().unwrap_or("");
                split_long_verse(text, budget)
            })
            .collect();

        let num_parts = all_parts.iter().map(|p| p.len()).max().unwrap_or(1);
        if num_parts > 1 {
            for (i, parts) in all_parts.iter_mut().enumerate() {
                if parts.len() < num_parts {
                    let text = texts[i].get(&key).copied().unwrap_or("");
                    *parts = split_verse_into_n(text, num_parts);
                }
            }

            return (0..num_parts)
                .map(|part_idx| {
                    let columns = translations
                        .iter()
                        .enumerate()
                        .map(|(t_idx, translation)| {
                            let part_text = all_parts[t_idx]
                                .get(part_idx)
                                .cloned()
                                .unwrap_or_default();
                            let line = VerseLine {
                                number: key.1,
                                text: part_text,
                                continued: part_idx > 0,
                            };
                            ScriptureColumn {
                                translation: translation.abbreviation.clone(),
                                verses: vec![line],
                            }
                        })
                        .collect();
                    scripture_slide(columns, caption.clone(), options)
                })
                .collect();
        }
    }

    let columns = translations
        .iter()
        .zip(texts)
        .map(|(translation, map)| ScriptureColumn {
            translation: translation.abbreviation.clone(),
            verses: chunk
                .iter()
                .map(|key| VerseLine {
                    number: key.1,
                    text: map.get(key).copied().unwrap_or("").to_owned(),
                    continued: false,
                })
                .collect(),
        })
        .collect();
    vec![scripture_slide(columns, caption, options)]
}

fn split_verse_into_n(text: &str, n: usize) -> Vec<String> {
    if n <= 1 || text.is_empty() {
        return vec![text.to_owned()];
    }
    let words: Vec<&str> = text.split(' ').collect();
    if words.len() <= n {
        let mut res: Vec<String> = words.iter().map(|&w| w.to_owned()).collect();
        while res.len() < n {
            res.push(String::new());
        }
        return res;
    }

    let words_per_part = (words.len() + n - 1) / n;
    let mut parts = Vec::new();
    for chunk in words.chunks(words_per_part) {
        parts.push(chunk.join(" "));
    }
    while parts.len() < n {
        parts.push(String::new());
    }
    parts
}

fn scripture_slide(
    columns: Vec<ScriptureColumn>,
    caption: String,
    options: ScriptureOptions,
) -> Slide {
    let text = columns
        .first()
        .map(|column| plain_text(&column.verses, options.verse_numbers.unwrap_or(true)))
        .unwrap_or_default();
    Slide {
        caption: Some(caption),
        scripture: Some(ScriptureSlide {
            columns,
            verse_numbers: options.verse_numbers,
        }),
        ..Slide::text(text)
    }
}

/// `"16 For God so loved… 17 For God sent…"`, for places that show plain
/// text (the stage display, thumbnails' accessible names).
fn plain_text(verses: &[VerseLine], numbers: bool) -> String {
    verses
        .iter()
        .map(|line| {
            if numbers && !line.continued {
                format!("{} {}", line.number, line.text)
            } else {
                line.text.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The passage covered by a chunk (all in one chapter), e.g. John 3:16–17.
fn chunk_passage(book: BookId, chunk: &[(u16, u16)]) -> Passage {
    let first = chunk[0];
    let last = chunk[chunk.len() - 1];
    Passage {
        book,
        ranges: vec![VerseRange {
            start: VersePoint {
                chapter: first.0,
                verse: Some(first.1),
            },
            end: VersePoint {
                chapter: last.0,
                verse: Some(last.1),
            },
        }],
    }
}

/// Splits a verse that is longer than `budget` characters into parts of
/// roughly that size, preferring to cut after punctuation, then between
/// words. Returns the verse unchanged if it fits.
fn split_long_verse(text: &str, budget: usize) -> Vec<String> {
    if text.chars().count() <= budget {
        return vec![text.to_owned()];
    }

    // Clauses end after punctuation followed by a space.
    let mut clauses: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
        if word.ends_with(['.', ';', ':', ',', '!', '?']) {
            clauses.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        clauses.push(current);
    }

    // Pack clauses into parts; a clause longer than the budget is split by words.
    let mut parts: Vec<String> = Vec::new();
    let mut part = String::new();
    for clause in clauses {
        let fits = part.chars().count() + 1 + clause.chars().count() <= budget;
        if !part.is_empty() && !fits {
            parts.push(std::mem::take(&mut part));
        }
        if clause.chars().count() > budget {
            for word in clause.split(' ') {
                if !part.is_empty() && part.chars().count() + 1 + word.chars().count() > budget {
                    parts.push(std::mem::take(&mut part));
                }
                if !part.is_empty() {
                    part.push(' ');
                }
                part.push_str(word);
            }
        } else {
            if !part.is_empty() {
                part.push(' ');
            }
            part.push_str(&clause);
        }
    }
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn john() -> BookId {
        BookId(43)
    }

    fn translation(abbreviation: &str, verses: &[(u16, u16, &str)]) -> TranslationText {
        TranslationText {
            bible_id: Id::new(),
            abbreviation: abbreviation.into(),
            copyright: "Public Domain".into(),
            verses: verses
                .iter()
                .map(|&(chapter, verse, text)| Verse {
                    book: john(),
                    chapter,
                    verse,
                    text: text.into(),
                })
                .collect(),
        }
    }

    fn passage() -> Passage {
        sw_bible::reference::parse("jn 3:16-18").unwrap().passage
    }

    fn captions(slides: &[Slide]) -> Vec<&str> {
        slides.iter().filter_map(|s| s.caption.as_deref()).collect()
    }

    #[test]
    fn groups_verses_by_count() {
        let kjv = translation("KJV", &[(3, 16, "a"), (3, 17, "b"), (3, 18, "c")]);
        let slides = slides(&passage(), &[kjv], ScriptureOptions::default());

        assert_eq!(captions(&slides), ["John 3:16–17 (KJV)", "John 3:18 (KJV)"]);
        assert_eq!(slides[0].text, "16 a 17 b");
    }

    #[test]
    fn never_mixes_chapters() {
        let kjv = translation("KJV", &[(3, 36, "a"), (4, 1, "b")]);
        let slides = slides(&passage(), &[kjv], ScriptureOptions::default());
        assert_eq!(captions(&slides), ["John 3:36 (KJV)", "John 4:1 (KJV)"]);
    }

    #[test]
    fn long_verses_get_a_slide_of_their_own() {
        let long = "x".repeat(320);
        let kjv = translation("KJV", &[(3, 16, "short"), (3, 17, &long)]);
        let slides = slides(&passage(), &[kjv], ScriptureOptions::default());
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn very_long_verses_split_at_punctuation() {
        let verse = "First clause is here, second clause follows; third clause ends it.";
        let parts = split_long_verse(verse, 30);
        assert_eq!(
            parts,
            [
                "First clause is here,",
                "second clause follows;",
                "third clause ends it."
            ]
        );

        let kjv = translation("KJV", &[(3, 16, verse)]);
        let options = ScriptureOptions {
            max_chars: 45,
            ..ScriptureOptions::default()
        };
        let slides = slides(&passage(), &[kjv], options);
        assert_eq!(slides.len(), 2);
        let second = &slides[1].scripture.as_ref().unwrap().columns[0].verses[0];
        assert!(second.continued, "later parts are marked as continued");
        assert!(
            !slides[1].text.starts_with("16"),
            "and show no verse number"
        );
    }

    #[test]
    fn parallel_translations_line_up_and_tolerate_gaps() {
        let kjv = translation("KJV", &[(3, 16, "kjv 16"), (3, 17, "kjv 17")]);
        let web = translation("WEB", &[(3, 16, "web 16")]);
        let slides = slides(&passage(), &[kjv, web], ScriptureOptions::default());

        assert_eq!(captions(&slides), ["John 3:16–17 (KJV · WEB)"]);
        let columns = &slides[0].scripture.as_ref().unwrap().columns;
        assert_eq!(columns[1].translation, "WEB");
        assert_eq!(columns[1].verses[1].text, "", "WEB lacks verse 17");
    }

    #[test]
    fn parallel_translations_split_long_verse_across_slides() {
        let long_verse_1 = "First clause in KJV, second clause in KJV; third clause in KJV.";
        let long_verse_2 = "First clause in WEB, second clause in WEB; third clause in WEB.";
        let kjv = translation("KJV", &[(3, 16, long_verse_1)]);
        let web = translation("WEB", &[(3, 16, long_verse_2)]);
        let options = ScriptureOptions {
            max_chars: 40,
            auto_flow: true,
            min_font_size: 36,
            ..ScriptureOptions::default()
        };
        let slides = slides(&passage(), &[kjv, web], options);
        assert!(slides.len() >= 2, "Parallel long verse was split into multiple slides");
        for slide in &slides {
            let cols = &slide.scripture.as_ref().unwrap().columns;
            assert_eq!(cols.len(), 2);
            assert_eq!(cols[0].verses.len(), 1);
            assert_eq!(cols[1].verses.len(), 1);
        }
    }

    #[test]
    fn footer_deduplicates_and_shortens_notices() {
        let kjv = translation("KJV", &[]);
        let web = translation("WEB", &[]);
        assert_eq!(
            footer(&[kjv.clone(), web]).as_deref(),
            Some("Public Domain")
        );

        let mut long = kjv;
        long.copyright = "word ".repeat(100);
        assert!(footer(&[long]).unwrap().ends_with('…'));
    }
}
