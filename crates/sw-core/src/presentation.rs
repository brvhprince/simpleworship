//! Presentations: a ready-to-show list of slides with a theme.
//!
//! Anything the operator can put on screen (a song, a block of typed text and,
//! in later milestones, scripture, images or PDF pages) is first turned into
//! a [`Presentation`]. From then on the engine does not care where the slides
//! came from.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use sw_bible::Passage;

use crate::id::Id;
use crate::media::{MediaItem, MediaKind};
use crate::output::RenderFrame;
use crate::scripture::{self, ScriptureOptions, TranslationText};
use crate::slides::{self, Slide, SlideOptions};
use crate::song::Song;
use crate::theme::{Background, MediaFit, Theme};
use crate::themes::{Look, ThemeLibrary};

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
    /// A Bible passage.
    Scripture {
        /// The passage.
        passage: Passage,
    },
    /// An image or video from the media library, shown on its own.
    Media {
        /// The media item's id.
        media_id: Id,
    },
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
    /// The theme every slide is drawn with (with any background override
    /// already applied).
    pub theme: Theme,
    /// The library theme `theme` came from, so edits to it can be shown
    /// straight away. `None` if it did not come from the library.
    pub theme_id: Option<Id>,
    /// A background chosen for this item only, re-applied when the theme
    /// is refreshed.
    pub background_override: Option<Background>,
    /// Small print shown under the lyrics, e.g. copyright and CCLI number.
    pub footer: Option<String>,
}

impl Presentation {
    /// Builds a presentation from a song, following its arrangement.
    pub fn from_song(song: &Song, look: impl Into<Look>, options: SlideOptions) -> Self {
        Self::new(
            PresentationSource::Song { song_id: song.id },
            song.title.clone(),
            slides::slides_from_sections(&song.ordered_sections(), options),
            look.into(),
            song.footer(),
        )
    }

    /// Builds a presentation from free text. Blank lines separate slides.
    pub fn from_text(title: impl Into<String>, text: &str, look: impl Into<Look>) -> Self {
        Self::new(
            PresentationSource::Text,
            title.into(),
            slides::slides_from_text(text, SlideOptions::default()),
            look.into(),
            None,
        )
    }

    /// Builds a one-slide presentation that shows an image or video on its
    /// own, fitted to the screen with black bars rather than cropped.
    pub fn from_media(media: &MediaItem, look: impl Into<Look>) -> Self {
        let background = match media.kind {
            MediaKind::Image => Background::Image {
                media: media.as_ref(),
                fit: MediaFit::Contain,
            },
            MediaKind::Video => Background::Video {
                media: media.as_ref(),
                fit: MediaFit::Contain,
                looping: false,
                play_audio: true,
            },
        };
        let mut look: Look = look.into();
        // Black around the picture, whatever the theme says.
        look.background_override = Some(background.clone());
        look.theme.background = background;

        let slide = Slide {
            text: String::new(),
            section: None,
            starts_section: false,
            caption: None,
            scripture: None,
        };
        Self::new(
            PresentationSource::Media { media_id: media.id },
            media.name.clone(),
            vec![slide],
            look,
            None,
        )
    }

    fn new(
        source: PresentationSource,
        title: String,
        slides: Vec<Slide>,
        look: Look,
        footer: Option<String>,
    ) -> Self {
        Self {
            id: Id::new(),
            source,
            title,
            slides,
            theme: look.theme,
            theme_id: look.theme_id,
            background_override: look.background_override,
            footer,
        }
    }

    /// The look this presentation was built with.
    pub fn look(&self) -> Look {
        Look {
            theme_id: self.theme_id,
            theme: self.theme.clone(),
            background_override: self.background_override.clone(),
        }
    }

    /// Picks up a newer version of this presentation's library theme.
    /// Returns whether anything changed.
    pub fn restyle(&mut self, library: &ThemeLibrary) -> bool {
        let Some(record) = self.theme_id.and_then(|id| library.get(id)) else {
            return false;
        };
        let look = self.look().restyled(&record.theme);
        if look.theme == self.theme {
            return false;
        }
        self.theme = look.theme;
        true
    }

    /// Builds a presentation from a Bible passage in one or more
    /// translations (shown side by side).
    pub fn from_scripture(
        passage: &Passage,
        translations: &[TranslationText],
        options: ScriptureOptions,
        look: impl Into<Look>,
    ) -> Self {
        Self::new(
            PresentationSource::Scripture {
                passage: passage.clone(),
            },
            scripture::title(passage, translations),
            scripture::slides(passage, translations, options),
            look.into(),
            scripture::footer(translations),
        )
    }

    /// The fully resolved frame for the slide at `index`, if it exists.
    pub fn frame(&self, index: usize) -> Option<RenderFrame> {
        let slide = self.slides.get(index)?;
        Some(RenderFrame {
            text: slide.text.clone(),
            scripture: slide.scripture.clone(),
            caption: slide.caption.clone(),
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
