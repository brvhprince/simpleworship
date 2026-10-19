//! Visual themes: how text and backgrounds look on the output screen.
//!
//! All sizes are in pixels on a **1920×1080 reference canvas**. Renderers scale
//! the whole canvas to the real screen, so a theme looks the same on a 720p
//! projector, a 4K TV and a small preview thumbnail.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A complete visual style for slides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
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
    /// How the footer (copyright / CCLI line) looks.
    pub footer: FooterStyle,
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
    /// A still image, scaled to cover the screen.
    Image {
        /// Absolute path to the image file.
        path: String,
    },
    /// A video, scaled to cover the screen.
    Video {
        /// Absolute path to the video file.
        path: String,
        /// Restart the video when it ends.
        #[serde(rename = "loop")]
        looping: bool,
        /// Play the video's audio track.
        play_audio: bool,
    },
}

/// Font and decoration for slide text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

/// Style of the footer line (copyright and CCLI information).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct FooterStyle {
    /// Show the footer at all.
    pub visible: bool,
    /// Font size in reference pixels.
    pub font_size: f32,
    /// CSS color.
    pub color: String,
}

impl Default for FooterStyle {
    fn default() -> Self {
        Self {
            visible: true,
            font_size: 28.0,
            color: "rgba(255, 255, 255, 0.75)".into(),
        }
    }
}
