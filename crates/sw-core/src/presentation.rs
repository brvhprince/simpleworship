//! Presentations: a ready-to-show list of slides with a theme.
//!
//! Anything the operator can put on screen (a song, a block of typed text and,
//! in later milestones, scripture, images or PDF pages) is first turned into
//! a [`Presentation`]. From then on the engine does not care where the slides
//! came from.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::output::RenderFrame;
use crate::slides::{self, Slide, SlideOptions};
use crate::song::Song;
use crate::theme::Theme;

/// Where a presentation's slides came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum PresentationSource {
    /// A song from the library.
    Song {
        /// The song's id.
        song_id: Id,
    },
    /// Free text typed by the operator.
    Text,
}

/// A titled list of slides, styled with one theme.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Presentation {
    /// Identifies this particular presentation instance.
    pub id: Id,
    /// Where the slides came from.
    pub source: PresentationSource,
    /// Title shown in the operator UI, e.g. the song title.
    pub title: String,
    /// The slides, in the order they are shown.
    pub slides: Vec<Slide>,
    /// The theme every slide is drawn with.
    pub theme: Theme,
    /// Small print shown under the lyrics, e.g. copyright and CCLI number.
    pub footer: Option<String>,
}

impl Presentation {
    /// Builds a presentation from a song, following its arrangement.
    pub fn from_song(song: &Song, theme: Theme, options: SlideOptions) -> Self {
        Self {
            id: Id::new(),
            source: PresentationSource::Song { song_id: song.id },
            title: song.title.clone(),
            slides: slides::slides_from_sections(&song.ordered_sections(), options),
            theme,
            footer: song.footer(),
        }
    }

    /// Builds a presentation from free text. Blank lines separate slides.
    pub fn from_text(title: impl Into<String>, text: &str, theme: Theme) -> Self {
        Self {
            id: Id::new(),
            source: PresentationSource::Text,
            title: title.into(),
            slides: slides::slides_from_text(text, SlideOptions::default()),
            theme,
            footer: None,
        }
    }

    /// The fully resolved frame for the slide at `index`, if it exists.
    pub fn frame(&self, index: usize) -> Option<RenderFrame> {
        let slide = self.slides.get(index)?;
        Some(RenderFrame {
            text: slide.text.clone(),
            footer: self.footer.clone(),
            theme: self.theme.clone(),
        })
    }

    /// Index of the first slide of the section a shortcut such as `"V2"` or
    /// `"C"` refers to.
    pub fn find_section(&self, shortcut: &str) -> Option<usize> {
        let wanted = crate::lyrics::SectionLabel::parse(shortcut);
        self.slides.iter().position(|slide| {
            slide.starts_section && slide.section.as_ref().is_some_and(|l| l.matches(&wanted))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song() -> Song {
        let mut song = Song::new("Test Song");
        song.lyrics = "[Verse 1]\nOne\n\nOne b\n[Chorus]\nRefrain\n[Verse 2]\nTwo".into();
        song.copyright = "Public Domain".into();
        song
    }

    #[test]
    fn song_presentations_carry_slides_and_footer() {
        let p = Presentation::from_song(&song(), Theme::default(), SlideOptions::default());

        assert_eq!(p.title, "Test Song");
        assert_eq!(p.slides.len(), 4);
        assert_eq!(p.footer.as_deref(), Some("Public Domain"));
        assert_eq!(p.frame(1).unwrap().text, "One b");
        assert_eq!(p.frame(99), None);
    }

    #[test]
    fn finds_sections_by_shortcut() {
        let p = Presentation::from_song(&song(), Theme::default(), SlideOptions::default());

        assert_eq!(p.find_section("V1"), Some(0));
        assert_eq!(p.find_section("c"), Some(2));
        assert_eq!(p.find_section("V2"), Some(3));
        assert_eq!(
            p.find_section("V"),
            Some(0),
            "unnumbered picks the first verse"
        );
        assert_eq!(p.find_section("B"), None);
    }
}
