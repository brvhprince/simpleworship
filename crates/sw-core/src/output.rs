//! What output screens draw.
//!
//! Renderers (projector windows, stage displays, and later NDI or browser
//! outputs) never look at schedules or songs. They receive an [`OutputState`]
//! and draw it. Because the state is fully resolved, every renderer shows
//! exactly the same thing.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::alerts::Alert;
use crate::countdown::CountdownFrame;
use crate::deck::SlideElement;
use crate::id::Id;
use crate::markup::TextRun;
use crate::media::Logo;
use crate::scripture::ScriptureSlide;
use crate::stream::StreamOutput;
use crate::theme::Theme;
use crate::time::Timestamp;

/// One slide, resolved and ready to draw.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct RenderFrame {
    /// The slide text. Lines are separated by `\n`. For scripture this is a
    /// plain-text version; draw [`RenderFrame::scripture`] instead.
    pub text: String,
    /// Scripture verses in columns, when the slide is scripture.
    pub scripture: Option<ScriptureSlide>,
    /// A line shown with the slide, e.g. the reference "John 3:16–17 (KJV)".
    pub caption: Option<String>,
    /// The text in styled pieces, when it has bold, italic or underline.
    /// Draw this instead of `text` when it is there.
    #[serde(default)]
    pub rich: Option<Vec<TextRun>>,
    /// Text boxes, shapes and pictures, back to front, for a slide made in
    /// the canvas editor. Draw these instead of the text and caption.
    #[serde(default)]
    pub elements: Option<Vec<SlideElement>>,
    /// A live countdown, when the slide is one.
    #[serde(default)]
    pub countdown: Option<CountdownFrame>,
    /// Small print under the text, if any.
    pub footer: Option<String>,
    /// The theme to draw with.
    pub theme: Theme,
}

/// The operator's "panic buttons". Each one hides part of the output without
/// changing which slide is live.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct DisplayFlags {
    /// Show a black screen. Overrides everything else.
    pub black: bool,
    /// Show the church logo instead of the slide.
    pub logo: bool,
    /// Hide the slide text but keep the background.
    pub hide_text: bool,
    /// Hide the background but keep the text.
    pub hide_background: bool,
}

/// How the output changes from one frame to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Transition {
    /// The kind of transition.
    pub kind: TransitionKind,
    /// How long it takes, in milliseconds. Ignored for [`TransitionKind::Cut`].
    pub duration_ms: u32,
}

impl Default for Transition {
    /// A short fade, which looks smooth without feeling slow.
    fn default() -> Self {
        Self {
            kind: TransitionKind::Fade,
            duration_ms: 400,
        }
    }
}

/// The transition for each kind of content, as in EasyWorship: a church
/// might cut straight to scripture during preaching but fade between song
/// slides. Saved by the host, so it is the same after a restart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "bindings.ts")]
pub struct TransitionSettings {
    /// For songs.
    pub songs: Transition,
    /// For Bible passages.
    pub scripture: Transition,
    /// For presentations, imported PDFs and slideshows.
    pub presentations: Transition,
    /// For everything else: typed text, pictures, videos, countdowns.
    pub other: Transition,
}

/// The available transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum TransitionKind {
    /// Switch instantly.
    Cut,
    /// Cross-fade from the old frame to the new one.
    Fade,
}

/// Everything an output screen needs to draw right now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct OutputState {
    /// The live slide, or `None` when nothing is live (the screen is black).
    pub frame: Option<RenderFrame>,
    /// Which parts of the output are hidden.
    pub flags: DisplayFlags,
    /// How to animate into this state.
    pub transition: Transition,
    /// Playback of the live video background, if there is one.
    pub video: Option<VideoPlayback>,
    /// The church logo, shown when [`DisplayFlags::logo`] is on. Without
    /// one, the Logo button shows the background with no text.
    pub logo: Option<Logo>,
    /// An alert over the slides, if one is showing.
    pub alert: Option<Alert>,
    /// What the stream output shows (see [`crate::stream`]).
    #[serde(default)]
    pub stream: StreamOutput,
    /// Increases by one every time the output changes. Lets renderers that
    /// connect late, or miss an event, tell whether they are up to date.
    #[ts(type = "number")]
    pub revision: u64,
}

/// Where the live video is, shared by every screen that shows it.
///
/// Instead of streaming the position many times a second, the engine says
/// "at wall-clock time `anchor`, the video was at `position_ms`, and it is
/// (or is not) playing". Each screen works out the current position from
/// its own clock with [`VideoPlayback::position_at`], so the projector, the
/// operator's live pane and any future remote screen stay in step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct VideoPlayback {
    /// The video this playback belongs to.
    pub media_id: Id,
    /// Whether the video is playing.
    pub playing: bool,
    /// The position, in milliseconds, at `anchor`.
    #[ts(type = "number")]
    pub position_ms: u64,
    /// When `position_ms` was measured (milliseconds since the Unix epoch).
    #[ts(type = "number")]
    pub anchor: Timestamp,
    /// Start again from the beginning at the end.
    #[serde(rename = "loop")]
    pub looping: bool,
    /// Silence the video's sound on every screen. Starts from the
    /// background's "play sound" setting; the operator can change it live.
    pub muted: bool,
}

impl VideoPlayback {
    /// The position in milliseconds at wall-clock time `now`. For a looping
    /// video this keeps counting past the end; the renderer wraps it using
    /// the video's real duration.
    pub fn position_at(&self, now: Timestamp) -> u64 {
        if self.playing {
            let elapsed = u64::try_from(now - self.anchor).unwrap_or(0);
            self.position_ms + elapsed
        } else {
            self.position_ms
        }
    }
}
