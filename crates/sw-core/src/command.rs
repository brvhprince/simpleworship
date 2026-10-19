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

use sw_bible::Passage;

use crate::engine::PaneState;
use crate::id::Id;
use crate::media::{MediaItem, MediaRef};
use crate::output::{OutputState, Transition};
use crate::presentation::Presentation;
use crate::schedule::{Schedule, ScheduleItem};
use crate::scripture::{ScriptureOptions, TranslationText};
use crate::song::Song;
use crate::themes::{ItemLook, ThemeLibrary};

/// A request to change the live state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum Command {
    /// Load a ready-made presentation into the preview pane, with its first
    /// slide selected. Prefer the `Preview…` commands below, which let the
    /// engine pick the theme.
    ShowInPreview {
        /// The presentation to load. Boxed because it is much larger than
        /// every other command; JSON and TypeScript see a plain object.
        presentation: Box<Presentation>,
    },
    /// Load a song from the library into the preview pane.
    PreviewSong {
        /// The song.
        song: Box<Song>,
    },
    /// Load free text into the preview pane; blank lines separate slides.
    PreviewText {
        /// Title shown in the operator window.
        title: String,
        /// The text.
        text: String,
    },
    /// Load a Bible passage into the preview pane.
    PreviewScripture {
        /// The passage.
        passage: Passage,
        /// Its text in each translation, shown side by side.
        translations: Vec<TranslationText>,
        /// How it is split into slides.
        options: ScriptureOptions,
    },
    /// Load an image or video from the media library into the preview pane.
    PreviewMedia {
        /// The media item.
        media: Box<MediaItem>,
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
    /// Set (or remove) the church logo shown by the Logo button.
    SetLogoImage {
        /// The logo image, or `None` to show the background instead.
        logo: Option<MediaRef>,
    },

    // ── Video ────────────────────────────────────────────────────────────
    /// Play the live video.
    PlayVideo,
    /// Pause the live video.
    PauseVideo,
    /// Move the live video to a position.
    SeekVideo {
        /// Position in milliseconds.
        #[ts(type = "number")]
        position_ms: u64,
    },
    /// Turn looping of the live video on or off.
    SetVideoLoop {
        /// `true` to loop.
        on: bool,
    },
    /// Mute or unmute the live video's sound on every screen.
    SetVideoMuted {
        /// `true` to mute.
        on: bool,
    },

    // ── Themes ───────────────────────────────────────────────────────────
    /// Replace the theme library and defaults. Panes whose theme changed
    /// are redrawn with the new version.
    SetThemes {
        /// The themes and defaults.
        library: Box<ThemeLibrary>,
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
    /// Give an item its own theme or background (or remove them).
    SetScheduleItemLook {
        /// The item to change.
        id: Id,
        /// The new look.
        look: Box<ItemLook>,
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
    /// The theme library or the default themes changed.
    ThemesChanged {
        /// The new library.
        themes: ThemeLibrary,
    },
}
