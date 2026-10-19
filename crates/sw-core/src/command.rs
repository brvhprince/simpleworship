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

use crate::alerts::AlertRequest;
use crate::audio::AudioState;
use crate::deck::{AutoAdvance, DeckSlide, SlideDeck};
use crate::engine::PaneState;
use crate::id::Id;
use crate::media::{MediaItem, MediaRef};
use crate::output::{OutputState, Transition};
use crate::presentation::Presentation;
use crate::schedule::{Schedule, ScheduleItem};
use crate::scripture::{ScriptureOptions, TranslationText};
use crate::song::Song;
use crate::stage::{StageState, TimerMode};
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

    // ── Alerts ───────────────────────────────────────────────────────────
    /// Show an alert over the slides, replacing any alert already showing.
    ShowAlert {
        /// What to show and how.
        alert: AlertRequest,
    },
    /// Take the alert off the screen.
    ClearAlert,

    // ── Countdowns ───────────────────────────────────────────────────────
    /// A live countdown reached zero (sent by a screen showing it). If the
    /// countdown is set to continue, the next schedule item goes live.
    CountdownFinished,

    // ── Stage display ────────────────────────────────────────────────────
    /// Start a new timer on the stage display.
    StartStageTimer {
        /// Up or down.
        mode: TimerMode,
        /// For a count-down, its length in milliseconds.
        #[ts(type = "number")]
        duration_ms: u64,
    },
    /// Pause the stage timer.
    PauseStageTimer,
    /// Continue the stage timer after a pause.
    ResumeStageTimer,
    /// Remove the stage timer.
    ClearStageTimer,
    /// Show (or, with `None`, remove) a message on the stage display.
    SetStageMessage {
        /// The message.
        message: Option<String>,
    },

    // ── Background audio ─────────────────────────────────────────────────
    /// Replace the playlist; optionally start playing its first track.
    SetPlaylist {
        /// The tracks.
        tracks: Vec<MediaItem>,
        /// Start playing straight away.
        play: bool,
    },
    /// Play (or continue) the current track.
    PlayAudio,
    /// Pause the audio.
    PauseAudio,
    /// Play a track of the playlist from its start.
    PlayTrack {
        /// The track's position in the playlist.
        index: usize,
    },
    /// Go to the next (`forward`) or previous track.
    SkipTrack {
        /// `true` for next, `false` for previous.
        forward: bool,
    },
    /// The current track finished (sent by the player): go on to the next.
    TrackEnded,
    /// Move within the current track.
    SeekAudio {
        /// Position in milliseconds.
        #[ts(type = "number")]
        position_ms: u64,
    },
    /// Change the volume (0 silent, 1 full).
    SetVolume {
        /// The volume.
        volume: f32,
    },
    /// Turn looping of the playlist on or off.
    SetAudioLoop {
        /// `true` to loop.
        on: bool,
    },
    /// Fade the audio out over `duration_ms`, then stop.
    FadeOutAudio {
        /// How long the fade takes.
        #[ts(type = "number")]
        duration_ms: u64,
    },

    // ── Settings the engine needs ────────────────────────────────────────
    /// The church's CCLI licence number, added to song footers (or `None`).
    SetCcliLicense {
        /// The licence number.
        license: Option<String>,
    },
    /// This computer's time zone, as minutes ahead of UTC, for countdowns
    /// to a time of day.
    SetUtcOffset {
        /// Minutes ahead of UTC (e.g. 60 for UTC+1).
        minutes: i32,
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
    /// A presentation was edited in the library: update its copies in the
    /// schedule and any pane showing it.
    RefreshDeck {
        /// The saved deck.
        deck: Box<SlideDeck>,
    },

    // ── Slides inside a schedule item ────────────────────────────────────
    /// Adds slides to the end of a schedule item, e.g. pictures dropped onto
    /// it. Pictures, videos, typed text and presentations can take more
    /// slides; the item becomes a presentation of its own.
    AppendSlides {
        /// The schedule item.
        id: Id,
        /// The slides to add.
        slides: Vec<DeckSlide>,
    },
    /// Moves one slide of a schedule item's presentation.
    MoveItemSlide {
        /// The schedule item.
        id: Id,
        /// The slide's position now.
        from: usize,
        /// Where it goes.
        to: usize,
    },
    /// Removes one slide of a schedule item's presentation.
    RemoveItemSlide {
        /// The schedule item.
        id: Id,
        /// The slide's position.
        index: usize,
    },
    /// Turns moving to the next slide by itself on (or off, with `None`)
    /// for one schedule item.
    SetItemAutoAdvance {
        /// The schedule item.
        id: Id,
        /// How long each slide stays, and whether to loop.
        auto_advance: Option<AutoAdvance>,
    },
    /// Time is up for the live slide of an auto-advancing presentation. Sent
    /// by the operator window, which keeps time. Ignored if the operator has
    /// moved on in the meantime.
    AutoAdvance {
        /// The live presentation the timer was started for.
        presentation_id: Id,
        /// The slide it was started for.
        from: usize,
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
    /// The stage display's timer or message changed.
    StageChanged {
        /// The new stage state.
        stage: StageState,
    },
    /// The background audio changed.
    AudioChanged {
        /// The new audio state.
        audio: AudioState,
    },
    /// A song went live, for the CCLI usage report.
    SongWentLive {
        /// The song.
        song_id: Id,
        /// Its title, in case the song is not in this computer's library.
        title: String,
    },
}
