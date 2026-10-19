//! What output screens draw.
//!
//! Renderers (projector windows, stage displays, and later NDI or browser
//! outputs) never look at schedules or songs. They receive an [`OutputState`]
//! and draw it. Because the state is fully resolved, every renderer shows
//! exactly the same thing.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::theme::Theme;

/// One slide, resolved and ready to draw.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct RenderFrame {
    /// The slide text. Lines are separated by `\n`.
    pub text: String,
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
    /// Increases by one every time the output changes. Lets renderers that
    /// connect late, or miss an event, tell whether they are up to date.
    #[ts(type = "number")]
    pub revision: u64,
}
