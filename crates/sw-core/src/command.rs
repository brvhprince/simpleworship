//! The engine's public vocabulary: commands in, events out.
//!
//! Every change to what is on screen happens by sending a [`Command`] to
//! [`Engine::dispatch`](crate::engine::Engine::dispatch), which answers with
//! the [`EngineEvent`]s describing what changed. The desktop UI sends commands over
//! Tauri IPC today. A phone remote or a second operator computer can send the
//! very same commands over the network later.
//!
//! Toggle-style controls take the *desired* value (`SetBlack { on: true }`)
//! instead of "toggle". Two operators pressing "black" at the same time then
//! agree on the result.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::engine::PaneState;
use crate::id::Id;
use crate::output::{OutputState, Transition};
use crate::presentation::Presentation;
use crate::schedule::{Schedule, ScheduleItem};
use crate::song::Song;

/// A request to change the live state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum Command {
    /// Load a presentation into the preview pane, with its first slide selected.
    ShowInPreview {
        /// The presentation to load. Boxed because it is much larger than
        /// every other command; JSON and TypeScript see a plain object.
        presentation: Box<Presentation>,
    },
    /// Select a slide in the preview pane.
    SelectPreviewSlide {
        /// Zero-based slide index.
        index: usize,
    },
    /// Send the preview pane's presentation live, starting at its selected slide.
    GoLive,
    /// Make a slide of the live presentation the live slide.
    SelectLiveSlide {
        /// Zero-based slide index.
        index: usize,
    },
    /// Advance the live presentation by one slide. On the last slide of a
    /// schedule item, continue with the first slide of the next item.
    NextSlide,
    /// Go back one slide. On the first slide of a schedule item, go back to
    /// the last slide of the previous item.
    PreviousSlide,
    /// Jump to the first slide of a section, e.g. `"V2"` or `"C"`. Does
    /// nothing if the live presentation has no such section.
    JumpToSection {
        /// A section shortcut.
        shortcut: String,
    },
    /// Take the live presentation off screen.
    ClearLive,
    /// Turn the black screen on or off.
    SetBlack {
        /// `true` to show black.
        on: bool,
    },
    /// Turn the logo screen on or off.
    SetLogo {
        /// `true` to show the logo.
        on: bool,
    },
    /// Hide or show the slide text.
    SetHideText {
        /// `true` to hide the text.
        on: bool,
    },
    /// Hide or show the background.
    SetHideBackground {
        /// `true` to hide the background.
        on: bool,
    },
    /// Change the transition used for subsequent changes.
    SetTransition {
        /// The new transition.
        transition: Transition,
    },

    // ── Schedule ─────────────────────────────────────────────────────────
    /// Replace the whole schedule, e.g. after opening a file or choosing
    /// New. What is on screen stays on screen.
    ReplaceSchedule {
        /// The new schedule.
        schedule: Box<Schedule>,
    },
    /// Add an item to the schedule.
    AddScheduleItem {
        /// The item to add.
        item: Box<ScheduleItem>,
        /// Where to insert it; `None` appends it at the end.
        index: Option<usize>,
    },
    /// Move an item so it ends up at position `to`.
    MoveScheduleItem {
        /// The item to move.
        id: Id,
        /// Its new, zero-based position.
        to: usize,
    },
    /// Remove an item from the schedule. Does nothing if it is already gone.
    RemoveScheduleItem {
        /// The item to remove.
        id: Id,
    },
    /// Change an item's operator notes.
    SetScheduleItemNotes {
        /// The item to change.
        id: Id,
        /// The new notes.
        notes: String,
    },
    /// Load a schedule item into the preview pane.
    PreviewScheduleItem {
        /// The item to preview.
        id: Id,
    },
    /// Send a schedule item live, from its first slide.
    ShowScheduleItem {
        /// The item to show.
        id: Id,
    },
    /// A song was edited in the library: update its copies in the schedule
    /// and, if it is in the preview or live pane, the slides there too.
    RefreshSong {
        /// The saved song.
        song: Box<Song>,
    },
}

/// A notification that part of the live state changed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum EngineEvent {
    /// The preview pane changed: a new presentation or a different selected slide.
    PreviewChanged {
        /// The new preview pane state.
        preview: Option<PaneState>,
    },
    /// The live pane changed.
    LiveChanged {
        /// The new live pane state.
        live: Option<PaneState>,
    },
    /// What the output screens should draw changed.
    OutputChanged {
        /// The new output state.
        output: OutputState,
    },
    /// The schedule changed: items added, moved, removed or edited.
    ScheduleChanged {
        /// The new schedule.
        schedule: Schedule,
    },
}
