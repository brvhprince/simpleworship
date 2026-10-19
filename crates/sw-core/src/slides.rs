//! Splitting text into slides.
//!
//! The rules match what operators expect from EasyWorship:
//!
//! 1. Each song section starts a new slide.
//! 2. A blank line inside a section starts a new slide.
//! 3. If [`SlideOptions::max_lines`] is set, long chunks are split further so
//!    no slide has more than that many lines.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::countdown::CountdownSlide;
use crate::lyrics::{Section, SectionLabel};
use crate::scripture::ScriptureSlide;
use crate::theme::Background;

/// One slide: the text shown on screen at a time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Slide {
    /// The text to display. Lines are separated by `\n`.
    pub text: String,
    /// The song section this slide belongs to, if any.
    pub section: Option<SectionLabel>,
    /// `true` for the first slide of a section. Jump-to-section shortcuts
    /// land on these slides, and the UI shows the section name above them.
    pub starts_section: bool,
    /// A line shown with the slide, e.g. the reference "John 3:16–17 (KJV)".
    pub caption: Option<String>,
    /// For scripture: the verses laid out in columns, one per translation.
    /// `text` then holds a plain-text version of the first column.
    pub scripture: Option<ScriptureSlide>,
    /// A background for this slide only (each page of a presentation is
    /// its own picture).
    #[serde(default)]
    pub background: Option<Background>,
    /// For a countdown slide: what it counts down to.
    #[serde(default)]
    pub countdown: Option<CountdownSlide>,
}

impl Slide {
    /// A slide with just text and nothing else.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            section: None,
            starts_section: false,
            caption: None,
            scripture: None,
            background: None,
            countdown: None,
        }
    }
}

/// Options that control how text is split into slides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct SlideOptions {
    /// The most lines a slide may hold. `None` means no limit: only sections
    /// and blank lines split slides.
    pub max_lines: Option<u32>,
}

/// Turns song sections into slides, in the given order.
pub fn slides_from_sections(sections: &[Section], options: SlideOptions) -> Vec<Slide> {
    sections
        .iter()
        .flat_map(|section| slides_for_section(section, options))
        .collect()
}

/// Splits free text (no section tags) into slides at blank lines.
pub fn slides_from_text(text: &str, options: SlideOptions) -> Vec<Slide> {
    let section = Section {
        label: None,
        text: text.trim().to_owned(),
    };
    slides_for_section(&section, options)
}

/// Splits one section into slides. Only the first one is marked as starting
/// the section.
fn slides_for_section(section: &Section, options: SlideOptions) -> Vec<Slide> {
    split_into_chunks(&section.text, options.max_lines)
        .into_iter()
        .enumerate()
        .map(|(index, text)| Slide {
            text,
            section: section.label.clone(),
            starts_section: index == 0,
            caption: None,
            scripture: None,
            background: None,
            countdown: None,
        })
        .collect()
}

/// Splits text at blank lines, then caps each chunk at `max_lines` lines.
fn split_into_chunks(text: &str, max_lines: Option<u32>) -> Vec<String> {
    // A limit of 0 would loop forever; treat it like "no limit".
    let max_lines = max_lines.filter(|&n| n > 0).map(|n| n as usize);

    paragraphs(text)
        .into_iter()
        .flat_map(|lines| match max_lines {
            Some(max) => lines.chunks(max).map(|chunk| chunk.join("\n")).collect(),
            None => vec![lines.join("\n")],
        })
        .collect()
}

/// Groups non-blank lines into paragraphs separated by one or more blank lines.
fn paragraphs(text: &str) -> Vec<Vec<&str>> {
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line.trim_end());
        }
    }
    if !current.is_empty() {
        paragraphs.push(current);
    }

    paragraphs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lyrics::parse_sections;

    fn texts(slides: &[Slide]) -> Vec<&str> {
        slides.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn each_section_and_blank_line_starts_a_slide() {
        let sections = parse_sections("[V1]\na\nb\n\nc\n[C]\nd");
        let slides = slides_from_sections(&sections, SlideOptions::default());

        assert_eq!(texts(&slides), ["a\nb", "c", "d"]);
        assert_eq!(
            slides.iter().map(|s| s.starts_section).collect::<Vec<_>>(),
            [true, false, true]
        );
        assert_eq!(slides[1].section.as_ref().unwrap().name, "Verse 1");
    }

    #[test]
    fn max_lines_splits_long_paragraphs() {
        let options = SlideOptions { max_lines: Some(2) };
        let slides = slides_from_text("1\n2\n3\n4\n5", options);
        assert_eq!(texts(&slides), ["1\n2", "3\n4", "5"]);
    }

    #[test]
    fn zero_max_lines_means_no_limit() {
        let options = SlideOptions { max_lines: Some(0) };
        assert_eq!(slides_from_text("1\n2\n3", options).len(), 1);
    }

    #[test]
    fn runs_of_blank_lines_never_create_empty_slides() {
        let slides = slides_from_text("\n\na\n\n\n   \nb\n\n", SlideOptions::default());
        assert_eq!(texts(&slides), ["a", "b"]);
    }
}
