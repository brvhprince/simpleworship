//! Scripture: Bible translations, verses, and turning a passage into slides.
//!
//! A passage can be shown in one translation, or in two side by side. The
//! verses are split into slides by these rules:
//!
//! 1. A slide never mixes chapters.
//! 2. A slide holds at most [`ScriptureOptions::max_verses`] verses.
//! 3. With [`ScriptureOptions::auto_flow`] (EasyWorship's "Automatically
//!    flow scripture on multiple slides"), a slide holds only as much text
//!    as the theme's text box fits at [`ScriptureOptions::min_font_size`]
//!    (see [`chars_per_column`]), so long passages move on to the next slide
//!    instead of shrinking below that size.
//! 4. A single verse that is too long on its own is then split at
//!    punctuation into several slides; the later parts are marked as
//!    continued and show no verse number. Side by side translations are
//!    split into the same number of parts, so the columns stay together.
//!
//! Translations are matched verse by verse using each one's own numbering.
//! A verse that one translation lacks shows as blank in that column.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use sw_bible::{BookId, Passage, VersePoint, VerseRange};
use ts_rs::TS;

use crate::id::Id;
use crate::slides::Slide;
use crate::theme::{CANVAS_HEIGHT, CANVAS_WIDTH, Placement, Theme};

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
    /// No longer used: how much fits now comes from the theme (see
    /// [`chars_per_column`]). Kept so schedules saved before still load.
    pub max_chars: u32,
    /// Show verse numbers before each verse, or `None` to do as the theme
    /// says (see [`VerseNumberStyle`](crate::theme::VerseNumberStyle)).
    #[serde(default)]
    pub verse_numbers: Option<bool>,
    /// Move text that does not fit the theme's text box at
    /// `min_font_size` on to more slides. Without it, everything up to
    /// `max_verses` goes on one slide and shrinks as far as it must.
    #[serde(default = "default_auto_flow")]
    pub auto_flow: bool,
    /// The smallest font size, in canvas pixels (1920 × 1080), the text may
    /// shrink to before the rest goes on the next slide. EasyWorship's
    /// "Minimum font size before breaking slide".
    #[serde(default = "default_min_font_size")]
    pub min_font_size: u32,
}

fn default_auto_flow() -> bool {
    true
}

/// Large enough to read from the back of a church on most screens.
const DEFAULT_MIN_FONT_SIZE: u32 = 48;

fn default_min_font_size() -> u32 {
    DEFAULT_MIN_FONT_SIZE
}

impl Default for ScriptureOptions {
    fn default() -> Self {
        Self {
            max_verses: 2,
            max_chars: 300,
            verse_numbers: None,
            auto_flow: true,
            min_font_size: DEFAULT_MIN_FONT_SIZE,
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

/// Splits a passage into slides drawn with `theme`. See the
/// [module docs](self) for the rules.
pub fn slides(
    passage: &Passage,
    translations: &[TranslationText],
    options: ScriptureOptions,
    theme: &Theme,
) -> Vec<Slide> {
    let budget = if options.auto_flow {
        chars_per_column(theme, options.min_font_size as f32, translations.len())
    } else {
        // Everything up to `max_verses` on one slide, shrunk to fit.
        usize::MAX
    };
    slides_with_budget(passage, translations, options, budget)
}

/// About how many characters of verse text fit in one column of the
/// theme's text box with the font at `min_font_size` (or the theme's own
/// size, if that is smaller or the theme never shrinks text).
///
/// This is an estimate from the box's size, the font size, the line height
/// and an average letter width, made in the core so every screen splits a
/// passage the same way. It errs towards less text per slide: a slide that
/// is a little emptier is better than words below the minimum size. The
/// screens still shrink a slide that turns out too full.
pub fn chars_per_column(theme: &Theme, min_font_size: f32, columns: usize) -> usize {
    /// Average letter width, as a share of the font size (wide fonts such
    /// as Montserrat are about 0.55; most are narrower).
    const LETTER_WIDTH: f32 = 0.55;
    /// Lines rarely end exactly at the edge: words wrap early.
    const WRAP_LOSS: f32 = 0.85;
    /// The space between side by side columns (`.columns` in the renderer).
    const COLUMN_GAP: f32 = 1.2;
    /// The translation's name above each side by side column (`.translation`).
    const COLUMN_HEADING: f32 = 0.45 * 1.2 + 0.45 * 0.4;
    /// The smallest budget, so even an odd theme shows some words per slide.
    const MIN_BUDGET: usize = 40;

    let text = &theme.text;
    let font = if theme.text_box.auto_fit {
        min_font_size.clamp(12.0, text.font_size.max(12.0))
    } else {
        text.font_size.max(12.0)
    };
    let columns = columns.max(1) as f32;

    let area = &theme.text_box;
    let width = CANVAS_WIDTH - area.left - area.right;
    let mut height = CANVAS_HEIGHT - area.top - area.bottom;
    // The reference and the copyright line can sit under the verses, in
    // the same box.
    for label in [&theme.caption, &theme.footer] {
        if label.visible && label.placement == Placement::AfterText {
            height -= label.font_size * 1.6;
        }
    }
    if columns > 1.0 {
        height -= COLUMN_HEADING * font;
    }

    let column_width = (width - (columns - 1.0) * COLUMN_GAP * font) / columns;
    let mut letter = LETTER_WIDTH * font;
    if text.uppercase {
        letter *= 1.15;
    }
    if text.font_weight >= 600 {
        letter *= 1.06;
    }
    let per_line = (column_width / letter).floor().max(0.0);
    let lines = (height / (font * text.line_height.max(0.8)))
        .floor()
        .max(1.0);
    ((per_line * lines * WRAP_LOSS) as usize).max(MIN_BUDGET)
}

/// [`slides`] with an explicit number of characters per column.
fn slides_with_budget(
    passage: &Passage,
    translations: &[TranslationText],
    options: ScriptureOptions,
    per_column_budget: usize,
) -> Vec<Slide> {
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
    let all_keys: Vec<(u16, u16)> = keys.iter().copied().collect();
    let caption_passage = passage_from_keys(passage.book, &all_keys);

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
    let mut chunk_length: usize = 0;
    let mut flush = |chunk: &mut Vec<(u16, u16)>, chunk_length: &mut usize| {
        if !chunk.is_empty() {
            slides.extend(build_slides(
                &caption_passage,
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
        let full = chunk.len() >= max_verses
            || chunk_length.saturating_add(verse_length) > per_column_budget;
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
    passage: &Passage,
    chunk: &[(u16, u16)],
    translations: &[TranslationText],
    texts: &[HashMap<(u16, u16), &str>],
    budget: usize,
    options: ScriptureOptions,
) -> Vec<Slide> {
    let caption = format!("{} ({})", passage, abbreviations(translations));

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
                    *parts = split_into(text, num_parts);
                }
            }

            return (0..num_parts)
                .map(|part_idx| {
                    let columns = translations
                        .iter()
                        .enumerate()
                        .map(|(t_idx, translation)| {
                            let part_text =
                                all_parts[t_idx].get(part_idx).cloned().unwrap_or_default();
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

/// Splits `text` into at most `parts` pieces, at punctuation where it can
/// (as [`split_long_verse`] does), padded with empty pieces to exactly
/// `parts`. Used for the other translations of a verse split across
/// slides, so each slide shows the matching part of every column.
fn split_into(text: &str, parts: usize) -> Vec<String> {
    let parts = parts.max(1);
    let length = text.chars().count();
    let mut budget = length.div_ceil(parts).max(1);
    let mut pieces = loop {
        let pieces = split_long_verse(text, budget);
        if pieces.len() <= parts {
            break pieces;
        }
        // Clauses rarely divide evenly: allow a little more per piece.
        budget += budget / 10 + 1;
    };
    pieces.resize(parts, String::new());
    pieces
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

/// The passage covered by a list of verses, preserving ranges across chapters.
fn passage_from_keys(book: BookId, keys: &[(u16, u16)]) -> Passage {
    if keys.is_empty() {
        return Passage {
            book,
            ranges: Vec::new(),
        };
    }
    let mut ranges = Vec::new();
    let mut current_range: Option<VerseRange> = None;

    for &(chapter, verse) in keys {
        match current_range.as_mut() {
            Some(range)
                if range.end.chapter == chapter
                    && range.end.verse.is_some_and(|v| verse == v + 1) =>
            {
                range.end.verse = Some(verse);
            }
            _ => {
                if let Some(r) = current_range.take() {
                    ranges.push(r);
                }
                current_range = Some(VerseRange {
                    start: VersePoint {
                        chapter,
                        verse: Some(verse),
                    },
                    end: VersePoint {
                        chapter,
                        verse: Some(verse),
                    },
                });
            }
        }
    }
    if let Some(r) = current_range {
        ranges.push(r);
    }
    Passage { book, ranges }
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

    /// The theme scripture is drawn with in these tests.
    fn theme() -> Theme {
        Theme::default()
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
        let slides = slides(&passage(), &[kjv], ScriptureOptions::default(), &theme());

        assert_eq!(
            captions(&slides),
            ["John 3:16–18 (KJV)", "John 3:16–18 (KJV)"]
        );
        assert_eq!(slides[0].text, "16 a 17 b");
    }

    #[test]
    fn never_mixes_chapters() {
        let kjv = translation("KJV", &[(3, 36, "a"), (4, 1, "b")]);
        let p = sw_bible::reference::parse("jn 3:36; 4:1").unwrap().passage;
        let slides = slides(&p, &[kjv], ScriptureOptions::default(), &theme());
        assert_eq!(
            captions(&slides),
            ["John 3:36; 4:1 (KJV)", "John 3:36; 4:1 (KJV)"]
        );
    }

    #[test]
    fn passage_range_is_preserved_on_all_slide_captions() {
        let esther = sw_bible::reference::parse("esther 8:7-10").unwrap().passage;
        let kjv = translation(
            "KJV",
            &[
                (8, 7, "verse 7"),
                (8, 8, "verse 8"),
                (8, 9, "verse 9"),
                (8, 10, "verse 10"),
            ],
        );
        let slides = slides(
            &esther,
            &[kjv],
            ScriptureOptions {
                max_verses: 1,
                ..ScriptureOptions::default()
            },
            &theme(),
        );
        assert_eq!(slides.len(), 4);
        for slide in &slides {
            assert_eq!(slide.caption.as_deref(), Some("Esther 8:7–10 (KJV)"));
        }
    }

    #[test]
    fn long_verses_get_a_slide_of_their_own() {
        let budget = chars_per_column(&theme(), DEFAULT_MIN_FONT_SIZE as f32, 1);
        // Fits a slide alone, but not after another verse.
        let long = "x".repeat(budget - 2);
        let kjv = translation("KJV", &[(3, 16, "short"), (3, 17, &long)]);
        let slides = slides(&passage(), &[kjv], ScriptureOptions::default(), &theme());
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
        let slides = slides_with_budget(&passage(), &[kjv], ScriptureOptions::default(), 45);
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
        let slides = slides(
            &passage(),
            &[kjv, web],
            ScriptureOptions::default(),
            &theme(),
        );

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
        let slides = slides_with_budget(&passage(), &[kjv, web], ScriptureOptions::default(), 40);
        assert!(slides.len() >= 2, "the long verse is split across slides");
        for slide in &slides {
            let cols = &slide.scripture.as_ref().unwrap().columns;
            assert_eq!(cols.len(), 2);
            assert_eq!(cols[0].verses.len(), 1);
            assert_eq!(cols[1].verses.len(), 1);
        }
    }

    #[test]
    fn the_other_translation_splits_at_punctuation_too() {
        let verse = "Jesus wept, and the people saw it; then they went home.";
        assert_eq!(
            split_into(verse, 2),
            ["Jesus wept, and the people saw it;", "then they went home."]
        );
        assert_eq!(split_into("Amen.", 3), ["Amen.", "", ""]);
    }

    #[test]
    fn a_smaller_minimum_font_fits_more_on_a_slide() {
        let theme = theme();
        let at_36 = chars_per_column(&theme, 36.0, 1);
        let at_60 = chars_per_column(&theme, 60.0, 1);
        assert!(at_36 > at_60, "{at_36} should be more than {at_60}");
    }

    #[test]
    fn side_by_side_columns_each_fit_less() {
        let theme = theme();
        let one = chars_per_column(&theme, 48.0, 1);
        let two = chars_per_column(&theme, 48.0, 2);
        assert!(
            two < one / 2 + 10,
            "{two} per column beside another, {one} alone"
        );
    }

    #[test]
    fn a_smaller_text_box_fits_less() {
        let mut lower_third = theme();
        lower_third.text_box.top = 700.0;
        let full = chars_per_column(&theme(), 48.0, 1);
        let small = chars_per_column(&lower_third, 48.0, 1);
        assert!(
            small < full / 2,
            "{small} in a lower third, {full} full screen"
        );
    }

    #[test]
    fn a_theme_that_never_shrinks_is_measured_at_its_own_size() {
        let mut fixed = theme();
        fixed.text_box.auto_fit = false;
        assert_eq!(
            chars_per_column(&fixed, 30.0, 1),
            chars_per_column(&theme(), fixed.text.font_size, 1)
        );
    }

    #[test]
    fn without_auto_flow_a_long_verse_stays_whole() {
        let long = "word, ".repeat(400);
        let kjv = translation("KJV", &[(3, 16, &long)]);
        let options = ScriptureOptions {
            auto_flow: false,
            ..ScriptureOptions::default()
        };
        assert_eq!(slides(&passage(), &[kjv], options, &theme()).len(), 1);
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
