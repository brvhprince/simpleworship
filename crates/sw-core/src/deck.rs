//! Slide decks: presentations made in SimpleWorship's own editor, imported
//! PDFs, and slideshows of pictures.
//!
//! A deck is an ordered list of slides. Each slide has text (with bold,
//! italic and underline, see [`crate::markup`]), a picture or video, or
//! both. A deck can have its own theme, and any slide can use a different
//! theme or background and adjust the text's size, colour and alignment.
//!
//! The library keeps decks for reuse. A schedule item holds its own copy,
//! like a song, so slides can be added to it or reordered for one service
//! without changing the library's deck.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::media::MediaItem;
use crate::presentation::shown_on_its_own;
use crate::theme::{Background, HorizontalAlign};
use crate::time::{self, Timestamp};

/// A presentation: a named, ordered list of slides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct SlideDeck {
    /// Unique identifier. Pages imported from a PDF are media items with
    /// this as their `collection`.
    pub id: Id,
    /// The name, e.g. "Announcements" or the PDF's file name.
    pub name: String,
    /// A theme for the whole deck, instead of the default for presentations.
    #[serde(default)]
    pub theme_id: Option<Id>,
    /// The slides, in order.
    pub slides: Vec<DeckSlide>,
    /// Move to the next slide by itself, e.g. for announcements before the
    /// service.
    #[serde(default)]
    pub auto_advance: Option<AutoAdvance>,
    /// When it was created.
    #[ts(type = "number")]
    pub created_at: Timestamp,
    /// When it was last changed.
    #[serde(default)]
    #[ts(type = "number")]
    pub updated_at: Timestamp,
}

/// One slide of a deck.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct DeckSlide {
    /// Identifies the slide, so the editor can keep track of it while it moves.
    pub id: Id,
    /// The text, with markup for bold, italic and underline. May be empty.
    #[serde(default)]
    pub text: String,
    /// A background for this slide only: a picture, video or colour.
    #[serde(default)]
    pub background: Option<Background>,
    /// A theme for this slide only, instead of the deck's.
    #[serde(default)]
    pub theme_id: Option<Id>,
    /// Adjustments to the theme's text for this slide.
    #[serde(default)]
    pub style: SlideStyle,
}

/// Changes to the theme's text style for one slide. `None` keeps the theme's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct SlideStyle {
    /// Font size in reference pixels.
    pub font_size: Option<f32>,
    /// CSS colour of the text.
    pub color: Option<String>,
    /// Horizontal alignment.
    pub align: Option<HorizontalAlign>,
}

impl SlideStyle {
    /// Whether it changes nothing.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Moving through a deck by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct AutoAdvance {
    /// Seconds each slide stays on screen.
    pub seconds: u32,
    /// After the last slide, start again from the first.
    #[serde(rename = "loop")]
    pub looping: bool,
}

impl SlideDeck {
    /// An empty deck.
    pub fn new(name: impl Into<String>) -> Self {
        let now = time::now();
        Self {
            id: Id::new(),
            name: name.into(),
            theme_id: None,
            slides: Vec::new(),
            auto_advance: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// A slideshow: one slide per picture or video. Sound files are left out.
    pub fn slideshow(name: impl Into<String>, media: &[MediaItem]) -> Self {
        Self {
            slides: media.iter().filter_map(DeckSlide::picture).collect(),
            ..Self::new(name)
        }
    }

    /// A name for a slideshow of `media`: the picture's name for one, or
    /// "Slideshow (10 pictures)".
    pub fn slideshow_name(media: &[MediaItem]) -> String {
        match media {
            [one] => one.name.clone(),
            many => format!("Slideshow ({} pictures)", many.len()),
        }
    }
}

impl DeckSlide {
    /// A slide with just text.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            text: text.into(),
            background: None,
            theme_id: None,
            style: SlideStyle::default(),
        }
    }

    /// A slide showing a picture or video on its own, fitted to the screen.
    /// `None` for sound files, which have nothing to show.
    pub fn picture(media: &MediaItem) -> Option<Self> {
        Some(Self {
            background: Some(shown_on_its_own(media)?),
            ..Self::text("")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::MediaKind;

    fn media(name: &str, kind: MediaKind) -> MediaItem {
        MediaItem {
            id: Id::new(),
            kind,
            name: name.into(),
            file: format!("{name}.bin"),
            width: None,
            height: None,
            duration_ms: None,
            size_bytes: 1,
            created_at: 0,
            collection: None,
        }
    }

    #[test]
    fn slideshows_show_pictures_and_videos_but_not_sound() {
        let files = [
            media("One", MediaKind::Image),
            media("Two", MediaKind::Video),
            media("Hymn", MediaKind::Audio),
        ];
        let deck = SlideDeck::slideshow(SlideDeck::slideshow_name(&files), &files);
        assert_eq!(deck.slides.len(), 2);
        assert_eq!(deck.name, "Slideshow (3 pictures)");
        assert_eq!(SlideDeck::slideshow_name(&files[..1]), "One");
    }

    #[test]
    fn old_decks_without_new_fields_still_load() {
        let json = r#"{"id":"01920c7e-0000-7000-8000-000000000001","name":"A",
            "slides":[{"id":"01920c7e-0000-7000-8000-000000000002","text":"Hi"}],
            "createdAt":5}"#;
        let deck: SlideDeck = serde_json::from_str(json).unwrap();
        assert!(deck.slides[0].style.is_empty());
        assert_eq!(deck.auto_advance, None);
    }
}
