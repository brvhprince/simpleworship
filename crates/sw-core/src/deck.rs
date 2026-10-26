//! Slide decks: presentations made in SimpleWorship's own editor, imported
//! PDFs, and slideshows of pictures.
//!
//! A deck is an ordered list of slides. Each slide is a canvas of
//! [`SlideElement`]s, drawn back to front: text boxes (any words can have
//! their own font, size, colour, bold, italic or underline), shapes, and
//! pictures or videos, each placed and sized anywhere on the 1920×1080
//! canvas. Behind them is the slide's background: its own, or the theme's.
//!
//! A deck can have its own theme, and any slide can use a different one.
//! Elements that are "the same" on several slides (the title box, the
//! logo) share a `role`, so a change can be applied to all slides at once.
//!
//! Slides made before the canvas editor have plain `text` instead of
//! elements; they are drawn in the theme's text box, and the editor turns
//! them into a text element when opened ([`DeckSlide::upgrade`]).
//!
//! The library keeps decks for reuse. A schedule item holds its own copy,
//! like a song, so slides can be added to it or reordered for one service
//! without changing the library's deck.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::markup::{self, TextRun};
use crate::media::{MediaItem, MediaKind, MediaRef};
use crate::presentation::shown_on_its_own;
use crate::theme::{Background, HorizontalAlign, MediaFit, TextStyle, Theme, VerticalAlign};
use crate::time::{self, Timestamp};

/// The reference canvas, in pixels (see [`crate::theme`]).
const CANVAS_WIDTH: f32 = 1920.0;
const CANVAS_HEIGHT: f32 = 1080.0;

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
    /// A background for this slide only: a picture, video or colour.
    #[serde(default)]
    pub background: Option<Background>,
    /// A theme for this slide only, instead of the deck's.
    #[serde(default)]
    pub theme_id: Option<Id>,
    /// What is on the slide, back to front.
    #[serde(default)]
    pub elements: Vec<SlideElement>,
    /// Text of a slide made before the canvas editor, with markup for bold,
    /// italic and underline, drawn in the theme's text box. Empty for
    /// slides with elements.
    #[serde(default)]
    pub text: String,
    /// Adjustments to the theme's text for `text`.
    #[serde(default)]
    pub style: SlideStyle,
}

/// One thing on a slide: a text box, a shape or a picture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct SlideElement {
    /// Identifies the element on its slide.
    pub id: Id,
    /// Shared by "the same" element on different slides (the title box,
    /// the logo), so a change can be applied to all slides. Copied when a
    /// slide's layout is reused.
    pub role: Id,
    /// Where it is, in canvas pixels.
    pub frame: Frame,
    /// How see-through it is, from 0 (invisible) to 1 (solid).
    #[serde(default = "crate::theme::full_opacity")]
    pub opacity: f32,
    /// Locked elements cannot be moved or resized by accident.
    #[serde(default)]
    pub locked: bool,
    /// What it is.
    pub content: ElementContent,
}

/// A rectangle on the 1920×1080 canvas.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Frame {
    /// Distance from the left edge.
    pub x: f32,
    /// Distance from the top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// What a slide element is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum ElementContent {
    /// A text box.
    Text {
        /// The text, in styled pieces. Line breaks are `\n` in the text.
        runs: Vec<TextRun>,
        /// The box's own style; runs override parts of it.
        style: TextStyle,
        /// Where the text sits in the box, top to bottom.
        vertical_align: VerticalAlign,
        /// Shrink the text (from `style.font_size`) until it fits the box.
        auto_fit: bool,
        /// Grow short text past `style.font_size` until it fills the box
        /// (EasyWorship's "scale to fit"). Off unless the operator turns it
        /// on, so text keeps the size it was given.
        #[serde(default)]
        scale_up: bool,
        /// Space between the box's edge and its text, in canvas pixels.
        #[serde(default)]
        padding: f32,
        /// A colour behind the text, if any.
        #[serde(default)]
        fill: Option<String>,
        /// Rounded corners for the fill, in canvas pixels.
        #[serde(default)]
        corner_radius: f32,
    },
    /// A rectangle, ellipse or line.
    Shape {
        /// Which shape.
        shape: ShapeKind,
        /// The colour inside, if any.
        fill: Option<String>,
        /// The outline, if any.
        stroke: Option<Stroke>,
        /// Rounded corners for a rectangle, in canvas pixels.
        #[serde(default)]
        corner_radius: f32,
    },
    /// A picture or video from the media library.
    Media {
        /// The file.
        media: MediaRef,
        /// Picture or video.
        kind: MediaKind,
        /// How it fills its box.
        fit: MediaFit,
        /// Rounded corners, in canvas pixels.
        #[serde(default)]
        corner_radius: f32,
        /// For a video: play its sound on the projector.
        #[serde(default)]
        play_audio: bool,
    },
}

/// The shapes a slide can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum ShapeKind {
    /// A rectangle (with rounded corners if `corner_radius` is set).
    Rectangle,
    /// An ellipse filling the box.
    Ellipse,
    /// A straight line across the box's longer side.
    Line,
}

/// The outline of a shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Stroke {
    /// CSS colour.
    pub color: String,
    /// Width in canvas pixels.
    pub width: f32,
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
    /// A slide with just text (drawn in the theme's text box).
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            background: None,
            theme_id: None,
            elements: Vec::new(),
            text: text.into(),
            style: SlideStyle::default(),
        }
    }

    /// Turns a slide made before the canvas editor into one with elements:
    /// its text becomes a text box where the theme puts text, styled like
    /// the theme's text with the slide's adjustments. Does nothing to a
    /// slide that already has elements or has no text.
    pub fn upgrade(&mut self, theme: &Theme) {
        if !self.elements.is_empty() || self.text.trim().is_empty() {
            self.text.clear();
            return;
        }
        let mut style = theme.text.clone();
        if let Some(size) = self.style.font_size {
            style.font_size = size;
        }
        if let Some(color) = &self.style.color {
            style.color = color.clone();
        }
        if let Some(align) = self.style.align {
            style.align = align;
        }
        let area = &theme.text_box;
        let element = SlideElement::new(
            Frame {
                x: area.left,
                y: area.top,
                width: CANVAS_WIDTH - area.left - area.right,
                height: CANVAS_HEIGHT - area.top - area.bottom,
            },
            ElementContent::Text {
                runs: markup::parse(&self.text),
                style,
                vertical_align: area.vertical_align,
                auto_fit: area.auto_fit,
                scale_up: area.scale_up,
                padding: 0.0,
                fill: None,
                corner_radius: 0.0,
            },
        );
        self.elements.push(element);
        self.text.clear();
        self.style = SlideStyle::default();
    }

    /// The slide's words, for the stage display and the slide list: every
    /// text box's text, top to bottom, one per line.
    pub fn plain_text(&self) -> String {
        if self.elements.is_empty() {
            return markup::plain_text(&self.text);
        }
        let mut boxes: Vec<&SlideElement> = self
            .elements
            .iter()
            .filter(|element| matches!(element.content, ElementContent::Text { .. }))
            .collect();
        boxes.sort_by(|a, b| a.frame.y.total_cmp(&b.frame.y));
        boxes
            .iter()
            .filter_map(|element| match &element.content {
                ElementContent::Text { runs, .. } => {
                    Some(runs.iter().map(|run| run.text.as_str()).collect::<String>())
                }
                _ => None,
            })
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n")
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

impl SlideElement {
    /// A new element with fresh ids.
    pub fn new(frame: Frame, content: ElementContent) -> Self {
        Self {
            id: Id::new(),
            role: Id::new(),
            frame,
            opacity: 1.0,
            locked: false,
            content,
        }
    }

    /// The media file it shows, if it is a picture or video.
    pub fn media(&self) -> Option<&MediaRef> {
        match &self.content {
            ElementContent::Media { media, .. } => Some(media),
            _ => None,
        }
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

    #[test]
    fn old_text_slides_become_a_text_box_where_the_theme_puts_text() {
        let theme = Theme::default();
        let mut slide = DeckSlide::text("**Harvest** Sunday");
        slide.style.color = Some("gold".into());
        slide.upgrade(&theme);

        assert!(slide.text.is_empty());
        let [element] = slide.elements.as_slice() else {
            panic!("one text box")
        };
        assert_eq!(element.frame.x, theme.text_box.left);
        let ElementContent::Text { runs, style, .. } = &element.content else {
            panic!("a text box")
        };
        assert!(runs[0].bold);
        assert_eq!(style.color, "gold");
        assert_eq!(slide.plain_text(), "Harvest Sunday");
    }

    #[test]
    fn plain_text_reads_the_boxes_top_to_bottom() {
        let text = |y: f32, words: &str| {
            SlideElement::new(
                Frame {
                    x: 0.0,
                    y,
                    width: 100.0,
                    height: 50.0,
                },
                ElementContent::Text {
                    runs: vec![TextRun {
                        text: words.into(),
                        ..TextRun::default()
                    }],
                    style: TextStyle::default(),
                    vertical_align: VerticalAlign::Middle,
                    auto_fit: true,
                    scale_up: false,
                    padding: 0.0,
                    fill: None,
                    corner_radius: 0.0,
                },
            )
        };
        let slide = DeckSlide {
            elements: vec![text(500.0, "Body"), text(40.0, "PRAYER 1")],
            ..DeckSlide::text("")
        };
        assert_eq!(slide.plain_text(), "PRAYER 1\nBody");
    }
}
