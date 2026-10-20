//! The stream look: the same slides as the projector, drawn for a
//! livestream (EasyWorship's "Alternate Output").
//!
//! Usually the words sit low on the screen, over the camera, with nothing
//! behind them: OBS or vMix lays the stream output over the camera picture.
//! Each kind of content chooses its look:
//!
//! - **a theme** (by default the "Lower Third" theme, for songs and
//!   scripture): the slide is redrawn with it, on the stream's backdrop;
//! - **the same as the main output** (presentations and everything else):
//!   full-screen slides on the stream too;
//! - **hidden**: nothing on the stream.
//!
//! A camera background never reaches the stream: the stream's switcher has
//! the camera itself (and OBS on another computer could not open it), so
//! it becomes the stream's backdrop, and a lower third over the camera
//! shows as usual.
//!
//! The panic buttons keep the camera visible: Black, Logo and Clear Text
//! take the words off the stream, leaving the backdrop (usually see-through),
//! rather than putting black or the logo over the camera.
//!
//! Because the stream uses the very same slides, it can never be on a
//! different slide from the projector.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::output::{DisplayFlags, RenderFrame};
use crate::theme::Background;
use crate::themes::{ThemeLibrary, starter_ids};

/// How the stream shows one kind of content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum StreamLook {
    /// Redraw the slides with this theme, on the stream's backdrop.
    Theme {
        /// The theme, from the library. If it has been deleted, the stream
        /// shows the content as the main output does.
        theme_id: Id,
    },
    /// Exactly as on the main output.
    SameAsMain,
    /// Nothing on the stream.
    Hidden,
}

/// What is behind the words on the stream, where a theme is used (and
/// whenever nothing is showing).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum StreamBackdrop {
    /// See-through, so OBS or vMix shows the camera behind the words. Only
    /// the browser source and NDI carry transparency; a stream window on a
    /// screen shows black.
    #[default]
    Transparent,
    /// Chroma-key green, for hardware switchers that key out a colour.
    Green,
    /// Black.
    Black,
    /// The slide's own background, as on the main output.
    Main,
}

impl StreamBackdrop {
    /// The backdrop as a CSS colour (for [`StreamBackdrop::Main`], what shows
    /// when nothing is live).
    pub fn color(self) -> &'static str {
        match self {
            StreamBackdrop::Transparent => "transparent",
            StreamBackdrop::Green => "#00b140",
            StreamBackdrop::Black | StreamBackdrop::Main => "#000000",
        }
    }
}

/// The stream look, chosen in Settings › Stream and saved by the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct StreamSettings {
    /// Songs.
    pub songs: StreamLook,
    /// Bible passages.
    pub scripture: StreamLook,
    /// Presentations, imported PDFs and slideshows.
    pub presentations: StreamLook,
    /// Everything else: typed text, pictures, videos, countdowns.
    pub other: StreamLook,
    /// What is behind the words.
    pub backdrop: StreamBackdrop,
    /// Show alerts (nursery calls, notices) on the stream as well.
    pub show_alerts: bool,
}

impl Default for StreamSettings {
    fn default() -> Self {
        let lower_third = StreamLook::Theme {
            theme_id: starter_ids::LOWER_THIRD
                .parse()
                .expect("starter ids are valid UUIDs"),
        };
        Self {
            songs: lower_third,
            scripture: lower_third,
            presentations: StreamLook::SameAsMain,
            other: StreamLook::SameAsMain,
            backdrop: StreamBackdrop::default(),
            show_alerts: true,
        }
    }
}

/// The kinds of content that each have their own stream look (and their
/// own transition).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamKind {
    /// Songs.
    Songs,
    /// Bible passages.
    Scripture,
    /// Presentations and slideshows.
    Presentations,
    /// Everything else.
    Other,
}

impl StreamSettings {
    /// The look for `kind`.
    pub fn look(&self, kind: StreamKind) -> StreamLook {
        match kind {
            StreamKind::Songs => self.songs,
            StreamKind::Scripture => self.scripture,
            StreamKind::Presentations => self.presentations,
            StreamKind::Other => self.other,
        }
    }
}

/// What the stream draws right now. Part of
/// [`OutputState`](crate::output::OutputState); it shares the main output's
/// transition, video playback and alert.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct StreamOutput {
    /// The slide as the stream shows it, or `None` for just the backdrop.
    pub frame: Option<RenderFrame>,
    /// Hide the text but keep the slide's pictures (Clear Text on a slide
    /// made in the canvas editor).
    pub hide_text: bool,
    /// Leave out the slide's own background (Clear Background).
    pub hide_background: bool,
    /// The CSS colour behind everything.
    pub backdrop: String,
    /// Show the main output's alert.
    pub show_alerts: bool,
}

impl Default for StreamOutput {
    fn default() -> Self {
        Self {
            frame: None,
            hide_text: false,
            hide_background: false,
            backdrop: StreamBackdrop::default().color().to_owned(),
            show_alerts: true,
        }
    }
}

/// Works out what the stream shows for the live `frame` (of content of
/// `kind`), given the panic buttons.
pub fn stream_output(
    settings: &StreamSettings,
    themes: &ThemeLibrary,
    kind: Option<StreamKind>,
    frame: Option<&RenderFrame>,
    flags: DisplayFlags,
) -> StreamOutput {
    let backdrop = settings.backdrop;
    let mut output = StreamOutput {
        frame: None,
        hide_text: flags.hide_text,
        hide_background: flags.hide_background,
        backdrop: backdrop.color().to_owned(),
        show_alerts: settings.show_alerts,
    };
    // Black and Logo clear the stream instead of covering the camera.
    if flags.black || flags.logo {
        return output;
    }
    let (Some(kind), Some(frame)) = (kind, frame) else {
        return output;
    };

    let mut frame = frame.clone();
    match settings.look(kind) {
        StreamLook::Hidden => return output,
        StreamLook::SameAsMain => {}
        StreamLook::Theme { theme_id } => {
            if let Some(record) = themes.get(theme_id) {
                let own_background = frame.theme.background.clone();
                frame.theme = record.theme.clone();
                frame.theme.background = match backdrop {
                    StreamBackdrop::Main => own_background,
                    _ => Background::Color {
                        color: backdrop.color().to_owned(),
                    },
                };
            }
        }
    }

    if matches!(frame.theme.background, Background::Camera { .. }) {
        frame.theme.background = Background::Color {
            color: backdrop.color().to_owned(),
        };
    }

    // Clear Text on plain text leaves only the backdrop and background,
    // which is what the renderer does with `hide_text`. A themed slide's
    // background is the backdrop, so there is nothing else to show.
    output.frame = Some(frame);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::themes::starter_library;

    fn song_frame() -> RenderFrame {
        RenderFrame {
            text: "Amazing grace".into(),
            scripture: None,
            caption: None,
            rich: None,
            elements: None,
            countdown: None,
            footer: Some("CCLI 22025".into()),
            theme: Theme {
                name: "Default".into(),
                background: Background::Color {
                    color: "#123456".into(),
                },
                ..Theme::default()
            },
        }
    }

    fn show(kind: StreamKind, flags: DisplayFlags, settings: &StreamSettings) -> StreamOutput {
        stream_output(
            settings,
            &starter_library(),
            Some(kind),
            Some(&song_frame()),
            flags,
        )
    }

    #[test]
    fn songs_use_the_lower_third_on_a_see_through_backdrop() {
        let output = show(
            StreamKind::Songs,
            DisplayFlags::default(),
            &StreamSettings::default(),
        );
        let frame = output.frame.expect("a frame");
        assert_eq!(frame.text, "Amazing grace");
        assert_eq!(frame.theme.name, "Lower Third");
        assert_eq!(
            frame.theme.background,
            Background::Color {
                color: "transparent".into()
            }
        );
        assert_eq!(output.backdrop, "transparent");
    }

    #[test]
    fn presentations_look_the_same_as_the_main_output() {
        let output = show(
            StreamKind::Presentations,
            DisplayFlags::default(),
            &StreamSettings::default(),
        );
        assert_eq!(output.frame, Some(song_frame()));
    }

    #[test]
    fn black_and_logo_clear_the_stream_instead_of_covering_the_camera() {
        for flags in [
            DisplayFlags {
                black: true,
                ..DisplayFlags::default()
            },
            DisplayFlags {
                logo: true,
                ..DisplayFlags::default()
            },
        ] {
            let output = show(StreamKind::Songs, flags, &StreamSettings::default());
            assert_eq!(output.frame, None);
            assert_eq!(output.backdrop, "transparent");
        }
    }

    #[test]
    fn clear_text_is_passed_on() {
        let flags = DisplayFlags {
            hide_text: true,
            ..DisplayFlags::default()
        };
        let output = show(StreamKind::Songs, flags, &StreamSettings::default());
        assert!(output.hide_text);
    }

    #[test]
    fn hidden_content_and_nothing_live_show_only_the_backdrop() {
        let settings = StreamSettings {
            other: StreamLook::Hidden,
            backdrop: StreamBackdrop::Green,
            ..StreamSettings::default()
        };
        let hidden = show(StreamKind::Other, DisplayFlags::default(), &settings);
        assert_eq!(hidden.frame, None);
        assert_eq!(hidden.backdrop, "#00b140");

        let nothing = stream_output(
            &settings,
            &starter_library(),
            None,
            None,
            DisplayFlags::default(),
        );
        assert_eq!(nothing.frame, None);
    }

    #[test]
    fn the_main_backdrop_keeps_the_slides_own_background() {
        let settings = StreamSettings {
            backdrop: StreamBackdrop::Main,
            ..StreamSettings::default()
        };
        let frame = show(StreamKind::Songs, DisplayFlags::default(), &settings)
            .frame
            .expect("a frame");
        assert_eq!(frame.theme.name, "Lower Third");
        assert_eq!(frame.theme.background, song_frame().theme.background);
    }

    #[test]
    fn a_camera_background_becomes_the_backdrop() {
        let mut frame = song_frame();
        frame.theme.background = Background::Camera {
            camera: crate::media::CameraRef {
                device_id: "cam-1".into(),
                label: "Elgato HD60 S".into(),
            },
            fit: crate::theme::MediaFit::Cover,
            opacity: 1.0,
        };
        let settings = StreamSettings {
            songs: StreamLook::SameAsMain,
            ..StreamSettings::default()
        };
        let output = stream_output(
            &settings,
            &starter_library(),
            Some(StreamKind::Songs),
            Some(&frame),
            DisplayFlags::default(),
        );
        assert_eq!(
            output.frame.expect("a frame").theme.background,
            Background::Color {
                color: "transparent".into()
            }
        );
    }

    #[test]
    fn a_deleted_theme_falls_back_to_the_main_look() {
        let settings = StreamSettings {
            songs: StreamLook::Theme {
                theme_id: Id::new(),
            },
            ..StreamSettings::default()
        };
        let output = show(StreamKind::Songs, DisplayFlags::default(), &settings);
        assert_eq!(output.frame, Some(song_frame()));
    }
}
