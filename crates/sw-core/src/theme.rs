//! Visual themes: how text and backgrounds look on the output screen.
//!
//! All sizes are in pixels on a **1920×1080 reference canvas**. Renderers scale
//! the whole canvas to the real screen, so a theme looks the same on a 720p
//! projector, a 4K TV and a small preview thumbnail.
//!
//! Themes are stored as JSON. Every struct here has `#[serde(default)]`, so
//! a theme saved by an older version, which lacks a newer setting, still
//! loads: the missing setting simply takes its default value.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::media::MediaRef;

/// A complete visual style for slides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct Theme {
    /// Display name, e.g. "Default".
    pub name: String,
    /// What is drawn behind the text.
    pub background: Background,
    /// How the slide text looks.
    pub text: TextStyle,
    /// The area the text is laid out in.
    pub text_box: TextBox,
    /// How the footer (copyright / CCLI line) looks, and where it sits.
    pub footer: FooterStyle,
    /// How the scripture reference ("John 3:16 (KJV)") looks, and where it sits.
    pub caption: FooterStyle,
    /// Settings that only apply to scripture.
    pub scripture: ScriptureStyle,
}

impl Default for Theme {
    /// White, outlined sans-serif text over a dark blue gradient.
    fn default() -> Self {
        Self {
            name: "Default".into(),
            background: Background::LinearGradient {
                from: "#0b1a33".into(),
                to: "#000000".into(),
                angle_deg: 180.0,
            },
            text: TextStyle::default(),
            text_box: TextBox::default(),
            footer: FooterStyle::default(),
            caption: FooterStyle::caption(),
            scripture: ScriptureStyle::default(),
        }
    }
}

/// What is drawn behind the slide text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum Background {
    /// A solid CSS color, e.g. `"#000000"`.
    Color {
        /// The color.
        color: String,
    },
    /// A two-color linear gradient.
    LinearGradient {
        /// Start color.
        from: String,
        /// End color.
        to: String,
        /// Direction in degrees; 180 runs top to bottom.
        angle_deg: f32,
    },
    /// A still image.
    Image {
        /// The image in the media library.
        media: MediaRef,
        /// How the image fills the screen.
        #[serde(default)]
        fit: MediaFit,
    },
    /// A video. Playback (play, pause, position) is controlled from the live
    /// pane and shared by every screen; see [`crate::output::VideoPlayback`].
    Video {
        /// The video in the media library.
        media: MediaRef,
        /// How the video fills the screen.
        #[serde(default)]
        fit: MediaFit,
        /// Restart the video when it ends.
        #[serde(rename = "loop")]
        looping: bool,
        /// Play the video's audio track.
        play_audio: bool,
    },
}

impl Background {
    /// The media file this background shows, if it is an image or video.
    pub fn media(&self) -> Option<&MediaRef> {
        match self {
            Background::Image { media, .. } | Background::Video { media, .. } => Some(media),
            Background::Color { .. } | Background::LinearGradient { .. } => None,
        }
    }
}

/// How an image or video fills a screen of a different shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum MediaFit {
    /// Fill the whole screen, cropping the edges if needed. Best for
    /// backgrounds.
    #[default]
    Cover,
    /// Show all of it, with black bars if needed. Best for photos and
    /// videos shown on their own.
    Contain,
}

/// Font and decoration for slide text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct TextStyle {
    /// CSS font family list, e.g. `"Open Sans, Arial, sans-serif"`.
    pub font_family: String,
    /// Font size in reference pixels. With [`TextBox::auto_fit`] on, this is
    /// the *maximum* size.
    pub font_size: f32,
    /// CSS font weight, e.g. 400 (regular) or 700 (bold).
    pub font_weight: u16,
    /// Use italics.
    pub italic: bool,
    /// Show the text in capital letters.
    pub uppercase: bool,
    /// CSS color of the text.
    pub color: String,
    /// Line height as a multiple of the font size.
    pub line_height: f32,
    /// Horizontal alignment.
    pub align: HorizontalAlign,
    /// Outline drawn around each letter. Helps text stay readable on busy backgrounds.
    pub outline: Option<Outline>,
    /// Drop shadow behind the text.
    pub shadow: Option<Shadow>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "\"Open Sans\", \"Segoe UI\", Helvetica, Arial, sans-serif".into(),
            font_size: 84.0,
            font_weight: 600,
            italic: false,
            uppercase: false,
            color: "#ffffff".into(),
            line_height: 1.2,
            align: HorizontalAlign::Center,
            outline: Some(Outline {
                color: "#000000".into(),
                width: 2.0,
            }),
            shadow: Some(Shadow {
                color: "rgba(0, 0, 0, 0.8)".into(),
                offset_x: 3.0,
                offset_y: 3.0,
                blur: 6.0,
            }),
        }
    }
}

/// An outline drawn around each glyph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Outline {
    /// CSS color of the outline.
    pub color: String,
    /// Width in reference pixels.
    pub width: f32,
}

/// A drop shadow behind the text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Shadow {
    /// CSS color of the shadow (alpha allowed).
    pub color: String,
    /// Horizontal offset in reference pixels.
    pub offset_x: f32,
    /// Vertical offset in reference pixels.
    pub offset_y: f32,
    /// Blur radius in reference pixels.
    pub blur: f32,
}

/// Horizontal text alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum HorizontalAlign {
    /// Align to the left edge.
    Left,
    /// Center each line.
    Center,
    /// Align to the right edge.
    Right,
}

/// Vertical placement of the text block inside the text box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum VerticalAlign {
    /// Stick to the top.
    Top,
    /// Center vertically.
    Middle,
    /// Stick to the bottom (lower-third style).
    Bottom,
}

/// The rectangle the slide text is laid out in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct TextBox {
    /// Distance from the top edge, in reference pixels.
    pub top: f32,
    /// Distance from the right edge, in reference pixels.
    pub right: f32,
    /// Distance from the bottom edge, in reference pixels.
    pub bottom: f32,
    /// Distance from the left edge, in reference pixels.
    pub left: f32,
    /// Vertical placement of the text inside the box.
    pub vertical_align: VerticalAlign,
    /// Shrink the font (down from [`TextStyle::font_size`]) until the text fits.
    pub auto_fit: bool,
}

impl Default for TextBox {
    fn default() -> Self {
        Self {
            top: 80.0,
            right: 120.0,
            bottom: 140.0,
            left: 120.0,
            vertical_align: VerticalAlign::Middle,
            auto_fit: true,
        }
    }
}

/// Style and place of a line of small print: the footer (copyright and
/// CCLI information) or the scripture reference.
///
/// It sits in its own box that the theme designer lets the operator drag,
/// or straight after the slide text. Empty font and colour mean "the same
/// as the slide text".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct FooterStyle {
    /// Show it at all.
    pub visible: bool,
    /// Font size in reference pixels.
    pub font_size: f32,
    /// CSS color, or empty for the slide text's colour.
    pub color: String,
    /// CSS font family list, or empty for the slide text's font.
    pub font_family: String,
    /// Bold.
    pub bold: bool,
    /// Alignment, or `None` to line up with the slide text.
    pub align: Option<HorizontalAlign>,
    /// In its own box, or straight after the slide text.
    pub placement: Placement,
    /// Its box, when [`FooterStyle::placement`] is [`Placement::Box`].
    pub text_box: TextBox,
}

impl Default for FooterStyle {
    /// The footer: small and faint, along the bottom edge.
    fn default() -> Self {
        Self {
            visible: true,
            font_size: 28.0,
            color: "rgba(255, 255, 255, 0.75)".into(),
            font_family: String::new(),
            bold: false,
            align: None,
            placement: Placement::Box,
            text_box: TextBox {
                top: 990.0,
                right: 120.0,
                bottom: 26.0,
                left: 120.0,
                vertical_align: VerticalAlign::Bottom,
                auto_fit: true,
            },
        }
    }
}

impl FooterStyle {
    /// The scripture reference: larger than the footer, in the text's font
    /// and colour (the audience reads it), just above the footer.
    pub fn caption() -> Self {
        Self {
            font_size: 39.0,
            color: String::new(),
            bold: true,
            text_box: TextBox {
                top: 880.0,
                bottom: 70.0,
                ..Self::default().text_box
            },
            ..Self::default()
        }
    }
}

/// Where a line of small print goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum Placement {
    /// In its own box (see [`FooterStyle::text_box`]).
    Box,
    /// Straight after the slide text, inside the text box.
    AfterText,
}

/// Settings that only apply to scripture slides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct ScriptureStyle {
    /// How verse numbers look.
    pub verse_numbers: VerseNumberStyle,
    /// Show the reference on every slide of a passage, or only on the first.
    pub reference_on_every_slide: bool,
}

impl Default for ScriptureStyle {
    fn default() -> Self {
        Self {
            verse_numbers: VerseNumberStyle::default(),
            reference_on_every_slide: true,
        }
    }
}

/// How the verse numbers in scripture look.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct VerseNumberStyle {
    /// Show them. The Scripture tab can override this for one passage.
    pub visible: bool,
    /// CSS color, or empty for the verse text's colour (slightly faded).
    pub color: String,
    /// Size as a percentage of the verse text.
    pub size_percent: u16,
    /// Raised above the line (superscript), or in line with the text.
    pub superscript: bool,
    /// Bold.
    pub bold: bool,
}

impl Default for VerseNumberStyle {
    fn default() -> Self {
        Self {
            visible: true,
            color: String::new(),
            size_percent: 55,
            superscript: true,
            bold: false,
        }
    }
}
