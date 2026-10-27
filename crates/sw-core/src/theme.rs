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

use crate::media::{CameraRef, MediaRef};

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
    /// The kind of content it is made for, which the Themes tab files it
    /// under and the designer previews; `None` for themes from before
    /// categories (the Themes tab then goes by what it is the default for).
    #[serde(deserialize_with = "known_category")]
    pub category: Option<ThemeCategory>,
}

/// The kinds of content a theme can be made for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum ThemeCategory {
    /// Song lyrics.
    Song,
    /// Bible passages.
    Scripture,
    /// Presentations.
    Presentation,
    /// Quick text and announcements.
    Text,
}

/// Reads a theme's category, taking anything unknown (a theme from a newer
/// version, or one edited by hand) as none rather than refusing the theme.
fn known_category<'de, D>(deserializer: D) -> Result<Option<ThemeCategory>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}

impl Default for Theme {
    /// White, outlined sans-serif text over a dark blue gradient.
    fn default() -> Self {
        Self {
            name: "Default".into(),
            category: None,
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
        /// How the image fills the screen, as the theme or slide asks.
        #[serde(default)]
        fit: MediaFit,
        /// The file's own choice (its [`MediaScaling`](crate::MediaScaling)
        /// in the Media tab), which wins over `fit`. Filled in by the engine
        /// for the screens; `None` when the file is left on automatic.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        file_fit: Option<MediaFit>,
        /// How see-through it is, from 0 (invisible) to 1 (solid). Lower
        /// values fade it into black, so text on top stands out.
        #[serde(default = "full_opacity")]
        opacity: f32,
    },
    /// A camera or capture card, live. Video only (no sound).
    Camera {
        /// Which camera.
        camera: CameraRef,
        /// How the picture fills the screen.
        #[serde(default)]
        fit: MediaFit,
        /// How see-through it is, from 0 (invisible) to 1 (solid).
        #[serde(default = "full_opacity")]
        opacity: f32,
    },
    /// A video. Playback (play, pause, position) is controlled from the live
    /// pane and shared by every screen; see [`crate::output::VideoPlayback`].
    Video {
        /// The video in the media library.
        media: MediaRef,
        /// How the video fills the screen, as the theme or slide asks.
        #[serde(default)]
        fit: MediaFit,
        /// The file's own choice, which wins over `fit` (see
        /// [`Background::Image`]).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        file_fit: Option<MediaFit>,
        /// Restart the video when it ends.
        #[serde(rename = "loop")]
        looping: bool,
        /// Play the video's audio track.
        play_audio: bool,
        /// How see-through it is, from 0 (invisible) to 1 (solid). A
        /// dimmed video behind the words keeps them easy to read.
        #[serde(default = "full_opacity")]
        opacity: f32,
    },
}

impl Background {
    /// Records the file's own fit from `fits` (or clears it, for a file
    /// left on automatic). Only pictures and videos have one.
    pub fn apply_file_fits(&mut self, fits: &crate::FileFits) {
        if let Self::Image {
            media, file_fit, ..
        }
        | Self::Video {
            media, file_fit, ..
        } = self
        {
            *file_fit = fits.get(&media.id).copied();
        }
    }
}

/// The width of the canvas everything is laid out on, in reference pixels
/// (screens scale it to their own size).
pub(crate) const CANVAS_WIDTH: f32 = 1920.0;
/// The height of the canvas everything is laid out on.
pub(crate) const CANVAS_HEIGHT: f32 = 1080.0;

/// The default opacity: fully solid.
pub(crate) fn full_opacity() -> f32 {
    1.0
}

impl Background {
    /// The media file this background shows, if it is an image or video.
    pub fn media(&self) -> Option<&MediaRef> {
        match self {
            Background::Image { media, .. } | Background::Video { media, .. } => Some(media),
            Background::Color { .. }
            | Background::LinearGradient { .. }
            | Background::Camera { .. } => None,
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
    /// Stretch it to the exact shape, distorting it if needed.
    Stretch,
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
    /// Scale font up or down to auto fit container bounds.
    #[serde(default)]
    pub scale_up: bool,
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
            scale_up: false,
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
    /// The footer: small and faint, along the bottom edge, and off until a
    /// theme turns it on (a song's copyright and CCLI line, a translation's
    /// copyright).
    fn default() -> Self {
        Self {
            visible: false,
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
                scale_up: false,
            },
        }
    }
}

impl FooterStyle {
    /// The scripture reference: larger than the footer, in the text's font
    /// and colour (the audience reads it), just above the footer.
    pub fn caption() -> Self {
        Self {
            visible: true,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A theme saved with the given `category` JSON (or none).
    fn with_category(category: Option<&str>) -> Theme {
        let field = category.map_or(String::new(), |value| format!(r#","category":{value}"#));
        serde_json::from_str(&format!(r#"{{"name":"Test"{field}}}"#)).unwrap()
    }

    #[test]
    fn reads_the_known_categories() {
        assert_eq!(
            with_category(Some(r#""scripture""#)).category,
            Some(ThemeCategory::Scripture)
        );
        assert_eq!(
            with_category(Some(r#""text""#)).category,
            Some(ThemeCategory::Text)
        );
    }

    #[test]
    fn takes_a_missing_or_unknown_category_as_none() {
        assert_eq!(with_category(None).category, None);
        assert_eq!(with_category(Some("null")).category, None);
        assert_eq!(with_category(Some(r#""hymns""#)).category, None);
        assert_eq!(with_category(Some("3")).category, None);
    }

    #[test]
    fn saves_the_category_in_camel_case() {
        let theme = Theme {
            category: Some(ThemeCategory::Presentation),
            ..Theme::default()
        };
        let json = serde_json::to_value(&theme).unwrap();
        assert_eq!(json["category"], "presentation");
    }
}
