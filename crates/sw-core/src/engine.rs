//! The presentation engine: schedule, preview, live and output state.
//!
//! The engine mirrors the operator window:
//!
//! - **Schedule**: the order of service (see [`crate::schedule`]).
//! - **Preview**: what the operator is about to show. Changing it never
//!   affects the screens.
//! - **Live**: what the congregation sees right now.
//!
//! On top of the live slide sit the [`DisplayFlags`] (black, logo, hide text,
//! hide background). From the live slide and the flags the engine derives one
//! [`OutputState`], which is all a renderer needs.
//!
//! ## Running a service
//!
//! When a schedule item goes live, the *next* item is loaded into preview,
//! so the operator always sees what is coming. Pressing Next on the last
//! slide of an item continues with the next item, and Previous on the first
//! slide goes back to the previous one. A whole service can be run with one
//! key.
//!
//! ## Themes and video
//!
//! The engine holds the theme library and picks the theme for everything it
//! shows (see [`crate::themes`]), so the rule lives in one place. When the
//! live slide has a video background, the engine keeps its playback state
//! ([`VideoPlayback`]): a video carries on playing across slides that share
//! it, and starts from the beginning when a different video goes live.
//!
//! The engine is a plain struct with no I/O and no threads. The host (the
//! desktop app) owns it behind a mutex, feeds it [`Command`]s and broadcasts
//! the resulting [`EngineEvent`]s to every window. Its only contact with the
//! outside world is the clock it reads for video timing, which tests replace.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::alerts::Alert;
use crate::audio::{AudioState, Fade};
use crate::command::{Command, EngineEvent};
use crate::countdown::{CountdownEnd, CountdownFrame};
use crate::deck::SlideDeck;
use crate::id::Id;
use crate::media::{MediaItem, MediaKind, MediaRef};
use crate::output::{
    DisplayFlags, OutputState, RenderFrame, Transition, TransitionSettings, VideoPlayback,
};
use crate::presentation::{Presentation, PresentationSource};
use crate::schedule::{ItemContent, Schedule, ScheduleItem};
use crate::slides::SlideOptions;
use crate::song::Song;
use crate::stage::{StageState, StageTimer, TimerMode};
use crate::stream::{StreamKind, StreamOutput, StreamSettings, stream_output};
use crate::theme::Background;
use crate::themes::{ContentKind, ItemLook, ThemeLibrary};
use crate::time::{self, Timestamp};

/// A presentation loaded into a pane, plus which slide is selected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct PaneState {
    /// The loaded presentation.
    pub presentation: Presentation,
    /// Index of the selected slide. Always a valid index into
    /// `presentation.slides`, except for a presentation with no slides.
    pub selected: usize,
    /// The schedule item the presentation came from, if any.
    pub schedule_item: Option<Id>,
}

/// This computer's own operator window. Phones and other computers that
/// control SimpleWorship are operators too, each with their own preview
/// pane, identified by their device id (see [`Engine::dispatch_as`]).
pub const LOCAL_OPERATOR: Id = Id::nil();

/// A full copy of the engine state, sent to a window when it opens so it can
/// draw immediately without waiting for the next event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct EngineSnapshot {
    /// The order of service.
    pub schedule: Schedule,
    /// The themes and the default theme for each kind of content.
    pub themes: ThemeLibrary,
    /// The preview pane.
    pub preview: Option<PaneState>,
    /// The live pane.
    pub live: Option<PaneState>,
    /// What the output screens draw.
    pub output: OutputState,
    /// The stage display's timer and message.
    pub stage: StageState,
    /// The background audio.
    pub audio: AudioState,
    /// The transition for each kind of content.
    pub transitions: TransitionSettings,
    /// How the stream output shows each kind of content.
    pub stream: StreamSettings,
}

/// A command that could not be carried out.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum EngineError {
    /// The command needs a presentation in the preview pane.
    #[error("there is nothing in the preview pane")]
    NothingInPreview,
    /// The command needs a live presentation.
    #[error("nothing is live")]
    NothingLive,
    /// A slide index past the end of the presentation.
    #[error("slide {index} does not exist (the presentation has {count} slides)")]
    SlideOutOfRange {
        /// The requested index.
        index: usize,
        /// How many slides there are.
        count: usize,
    },
    /// A schedule item id that is not in the schedule (any more).
    #[error("that item is no longer in the schedule")]
    UnknownScheduleItem,
    /// Slides were added to an item that cannot take more (a song,
    /// scripture, a countdown or music).
    #[error("slides can only be added to pictures, videos, text and presentations")]
    CannotHoldSlides,
}

/// The presentation engine. See the [module docs](self).
#[derive(Debug)]
pub struct Engine {
    schedule: Schedule,
    /// Each operator's preview pane (see [`LOCAL_OPERATOR`]).
    previews: BTreeMap<Id, PaneState>,
    /// Who sent the command being carried out.
    operator: Id,
    live: Option<PaneState>,
    flags: DisplayFlags,
    /// The transition for each kind of content.
    transitions: TransitionSettings,
    /// How the stream output shows each kind of content.
    stream: StreamSettings,
    /// The themes and defaults everything is styled with.
    themes: ThemeLibrary,
    /// The church logo, if one is set.
    logo: Option<MediaRef>,
    /// Playback of the live video background, if there is one.
    video: Option<VideoPlayback>,
    /// The alert over the slides, if any.
    alert: Option<Alert>,
    /// The stage display's timer and message.
    stage: StageState,
    /// The background audio player.
    audio: AudioState,
    /// The church's CCLI licence number, added to song footers.
    ccli_license: Option<String>,
    /// This computer's time zone, in minutes ahead of UTC.
    utc_offset_minutes: i32,
    /// When the live countdown ends, and which presentation it belongs to.
    countdown_end: Option<(Id, Timestamp)>,
    /// Where the engine gets the time, for video, timers and countdowns.
    clock: fn() -> Timestamp,
    /// Set by any command that changes the schedule, so [`Engine::dispatch`]
    /// knows to announce it.
    schedule_changed: bool,
    /// The output state last announced to renderers.
    output: OutputState,
}

impl Default for Engine {
    fn default() -> Self {
        Self::with_clock(time::now)
    }
}

impl Default for OutputState {
    /// A black screen with nothing live.
    fn default() -> Self {
        Self {
            frame: None,
            flags: DisplayFlags::default(),
            transition: Transition::default(),
            video: None,
            logo: None,
            alert: None,
            stream: StreamOutput::default(),
            revision: 0,
        }
    }
}

/// Which slide to select when a schedule item is loaded into a pane.
#[derive(Clone, Copy)]
enum StartAt {
    First,
    Last,
}

impl Engine {
    /// Creates an engine with an empty schedule and nothing in preview or live.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an engine that reads the time from `clock`. Tests use this
    /// to control video timing.
    pub fn with_clock(clock: fn() -> Timestamp) -> Self {
        Self {
            schedule: Schedule::default(),
            previews: BTreeMap::new(),
            operator: LOCAL_OPERATOR,
            live: None,
            flags: DisplayFlags::default(),
            transitions: TransitionSettings::default(),
            stream: StreamSettings::default(),
            themes: ThemeLibrary::default(),
            logo: None,
            video: None,
            alert: None,
            stage: StageState::default(),
            audio: AudioState::default(),
            ccli_license: None,
            utc_offset_minutes: 0,
            countdown_end: None,
            clock,
            schedule_changed: false,
            output: OutputState::default(),
        }
    }

    /// A full copy of the current state, with this computer's own preview.
    pub fn snapshot(&self) -> EngineSnapshot {
        self.snapshot_for(LOCAL_OPERATOR)
    }

    /// A full copy of the current state, with `operator`'s preview.
    pub fn snapshot_for(&self, operator: Id) -> EngineSnapshot {
        EngineSnapshot {
            schedule: self.schedule.clone(),
            themes: self.themes.clone(),
            preview: self.previews.get(&operator).cloned(),
            live: self.live.clone(),
            output: self.output.clone(),
            stage: self.stage.clone(),
            audio: self.audio.clone(),
            transitions: self.transitions,
            stream: self.stream,
        }
    }

    /// The current schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// The theme library.
    pub fn themes(&self) -> &ThemeLibrary {
        &self.themes
    }

    /// Applies a command from this computer's own operator window (see
    /// [`Engine::dispatch_as`]).
    pub fn dispatch(&mut self, command: Command) -> Result<Vec<EngineEvent>, EngineError> {
        self.dispatch_as(LOCAL_OPERATOR, command)
    }

    /// Applies a command from `operator` and returns the events describing
    /// what changed. The preview pane a command uses is that operator's
    /// own; everything else (the schedule, what is live) is shared.
    ///
    /// Commands that change nothing (pressing "next" on the last slide,
    /// turning black on when it is already on) return no events.
    pub fn dispatch_as(
        &mut self,
        operator: Id,
        command: Command,
    ) -> Result<Vec<EngineEvent>, EngineError> {
        self.operator = operator;
        let result = self.dispatch_now(command);
        self.operator = LOCAL_OPERATOR;
        result
    }

    fn dispatch_now(&mut self, command: Command) -> Result<Vec<EngineEvent>, EngineError> {
        let before_previews = self.previews.clone();
        let before_live = self.live.clone();
        let before_stage = self.stage.clone();
        let before_audio = self.audio.clone();
        let before_transitions = self.transitions;
        let before_stream = self.stream;
        let themes_changing = matches!(command, Command::SetThemes { .. });
        self.schedule_changed = false;

        self.apply(command)?;

        let mut events = Vec::new();
        if themes_changing {
            events.push(EngineEvent::ThemesChanged {
                themes: self.themes.clone(),
            });
        }
        if self.schedule_changed {
            events.push(EngineEvent::ScheduleChanged {
                schedule: self.schedule.clone(),
            });
        }
        // Each operator whose preview changed (a theme edit can change
        // several).
        let operators: BTreeSet<Id> = before_previews
            .keys()
            .chain(self.previews.keys())
            .copied()
            .collect();
        for operator in operators {
            let preview = self.previews.get(&operator);
            if preview != before_previews.get(&operator) {
                events.push(EngineEvent::PreviewChanged {
                    operator,
                    preview: preview.cloned(),
                });
            }
        }
        if self.live != before_live {
            events.push(EngineEvent::LiveChanged {
                live: self.live.clone(),
            });
            events.extend(self.song_went_live(before_live.as_ref()));
        }
        if let Some(output) = self.refresh_output() {
            events.push(EngineEvent::OutputChanged {
                output: Box::new(output),
            });
        }
        if self.stage != before_stage {
            events.push(EngineEvent::StageChanged {
                stage: self.stage.clone(),
            });
        }
        if self.audio != before_audio {
            events.push(EngineEvent::AudioChanged {
                audio: self.audio.clone(),
            });
        }
        if self.transitions != before_transitions {
            events.push(EngineEvent::TransitionsChanged {
                transitions: self.transitions,
            });
        }
        if self.stream != before_stream {
            events.push(EngineEvent::StreamSettingsChanged {
                settings: self.stream,
            });
        }
        Ok(events)
    }

    /// Mutates the state for one command. Event bookkeeping is done by
    /// [`Engine::dispatch`].
    fn apply(&mut self, command: Command) -> Result<(), EngineError> {
        match command {
            // ── Preview and live ──────────────────────────────────────────
            Command::ShowInPreview { presentation } => {
                self.set_preview(unscheduled(*presentation));
            }
            Command::PreviewSong { song } => {
                let look = self.look(ContentKind::Song, song.theme_id);
                let presentation = Presentation::from_song(&song, look, SlideOptions::default());
                self.set_preview(unscheduled(self.finish(presentation)));
            }
            Command::PreviewText { title, text } => {
                let look = self.look(ContentKind::Text, None);
                self.set_preview(unscheduled(Presentation::from_text(title, &text, look)));
            }
            Command::PreviewScripture {
                passage,
                translations,
                options,
            } => {
                let look = self.look(ContentKind::Scripture, None);
                let presentation =
                    Presentation::from_scripture(&passage, &translations, options, look);
                self.set_preview(unscheduled(presentation));
            }
            Command::PreviewMedia { media } => {
                let look = self.look(ContentKind::Text, None);
                self.set_preview(unscheduled(Presentation::from_media(&media, look)));
            }
            Command::PreviewCamera { camera } => {
                let look = self.look(ContentKind::Text, None);
                self.set_preview(unscheduled(Presentation::from_camera(&camera, look)));
            }
            Command::SelectPreviewSlide { index } => {
                let pane = self
                    .previews
                    .get_mut(&self.operator)
                    .ok_or(EngineError::NothingInPreview)?;
                select(pane, index)?;
            }
            Command::GoLive => {
                let pane = self
                    .preview()
                    .cloned()
                    .ok_or(EngineError::NothingInPreview)?;
                self.go_live(pane);
                self.reveal();
            }
            Command::SelectLiveSlide { index } => {
                select(self.live_mut()?, index)?;
            }
            Command::NextSlide => self.next_slide()?,
            Command::PreviousSlide => self.previous_slide()?,
            Command::JumpToSection { shortcut } => {
                let pane = self.live_mut()?;
                if let Some(index) = pane.presentation.find_section(&shortcut) {
                    pane.selected = index;
                }
            }
            Command::ClearLive => self.live = None,

            // ── Display controls ──────────────────────────────────────────
            Command::SetBlack { on } => self.flags.black = on,
            Command::SetLogo { on } => self.flags.logo = on,
            Command::SetHideText { on } => self.flags.hide_text = on,
            Command::SetHideBackground { on } => self.flags.hide_background = on,
            Command::SetTransitions { transitions } => self.transitions = transitions,
            Command::SetStreamSettings { settings } => self.stream = settings,
            Command::SetLogoImage { logo } => self.logo = logo,

            // ── Video ─────────────────────────────────────────────────────
            Command::PlayVideo => self.update_video(|video, now| {
                if !video.playing {
                    video.anchor = now;
                    video.playing = true;
                }
            }),
            Command::PauseVideo => self.update_video(|video, now| {
                video.position_ms = video.position_at(now);
                video.anchor = now;
                video.playing = false;
            }),
            Command::SeekVideo { position_ms } => self.update_video(|video, now| {
                video.position_ms = position_ms;
                video.anchor = now;
            }),
            Command::SetVideoLoop { on } => self.update_video(|video, _| video.looping = on),
            Command::SetVideoMuted { on } => self.update_video(|video, _| video.muted = on),

            // ── Alerts and countdowns ─────────────────────────────────────
            Command::ShowAlert { alert } => {
                self.alert = Some(Alert {
                    request: alert,
                    started_at: (self.clock)(),
                });
            }
            Command::ClearAlert => self.alert = None,
            Command::CountdownFinished => self.finish_countdown()?,

            // ── Stage display ─────────────────────────────────────────────
            Command::StartStageTimer { mode, duration_ms } => {
                self.stage.timer = Some(StageTimer {
                    mode,
                    duration_ms: if mode == TimerMode::CountDown {
                        duration_ms
                    } else {
                        0
                    },
                    elapsed_ms: 0,
                    anchor: (self.clock)(),
                    running: true,
                });
            }
            Command::PauseStageTimer => {
                let now = (self.clock)();
                if let Some(timer) = &mut self.stage.timer
                    && timer.running
                {
                    timer.elapsed_ms = timer.elapsed_at(now);
                    timer.anchor = now;
                    timer.running = false;
                }
            }
            Command::ResumeStageTimer => {
                let now = (self.clock)();
                if let Some(timer) = &mut self.stage.timer
                    && !timer.running
                {
                    timer.anchor = now;
                    timer.running = true;
                }
            }
            Command::ClearStageTimer => self.stage.timer = None,
            Command::SetStageMessage { message } => {
                self.stage.message = message.filter(|text| !text.trim().is_empty());
            }

            // ── Background audio ──────────────────────────────────────────
            Command::SetPlaylist { tracks, play } => self.set_playlist(tracks, play),
            Command::PlayAudio => {
                let now = (self.clock)();
                if !self.audio.playing && self.audio.current().is_some() {
                    self.audio.anchor = now;
                    self.audio.playing = true;
                    self.audio.fade = None;
                }
            }
            Command::PauseAudio => {
                let now = (self.clock)();
                self.audio.position_ms = self.audio.position_at(now);
                self.audio.anchor = now;
                self.audio.playing = false;
                self.audio.fade = None;
            }
            Command::PlayTrack { index } => {
                if index < self.audio.playlist.len() {
                    self.audio.start_track(index, (self.clock)());
                }
            }
            Command::SkipTrack { forward } => self.skip_track(forward),
            Command::TrackEnded => match self.audio.next_index() {
                Some(next) => self.audio.start_track(next, (self.clock)()),
                None => {
                    self.audio.playing = false;
                    self.audio.position_ms = 0;
                    self.audio.fade = None;
                }
            },
            Command::SeekAudio { position_ms } => {
                self.audio.position_ms = position_ms;
                self.audio.anchor = (self.clock)();
            }
            Command::SetVolume { volume } => self.audio.volume = volume.clamp(0.0, 1.0),
            Command::SetAudioLoop { on } => self.audio.looping = on,
            Command::FadeOutAudio { duration_ms } => {
                if self.audio.playing {
                    self.audio.fade = Some(Fade {
                        started_at: (self.clock)(),
                        duration_ms,
                    });
                }
            }

            // ── Settings ──────────────────────────────────────────────────
            Command::SetCcliLicense { license } => {
                self.ccli_license = license.filter(|number| !number.trim().is_empty());
            }
            Command::SetUtcOffset { minutes } => self.utc_offset_minutes = minutes,

            // ── Themes ────────────────────────────────────────────────────
            Command::SetThemes { library } => {
                self.themes = *library;
                for pane in self.previews.values_mut().chain(self.live.as_mut()) {
                    pane.presentation.restyle(&self.themes);
                }
            }

            // ── Schedule ──────────────────────────────────────────────────
            Command::ReplaceSchedule { schedule } => {
                self.schedule = *schedule;
                self.schedule_changed = true;
            }
            Command::AddScheduleItem { item, index } => {
                self.schedule.insert(*item, index);
                self.schedule_changed = true;
            }
            Command::MoveScheduleItem { id, to } => {
                self.schedule_changed = self.schedule.move_item(id, to);
            }
            Command::RemoveScheduleItem { id } => {
                self.schedule_changed = self.schedule.remove(id);
            }
            Command::SetScheduleItemLook { id, look } => {
                let item = self
                    .schedule
                    .items
                    .iter_mut()
                    .find(|item| item.id == id)
                    .ok_or(EngineError::UnknownScheduleItem)?;
                if item.look != *look {
                    item.look = *look;
                    self.schedule_changed = true;
                    self.rebuild_panes_for(id);
                }
            }
            Command::SetScheduleItemNotes { id, notes } => {
                let item = self
                    .schedule
                    .items
                    .iter_mut()
                    .find(|item| item.id == id)
                    .ok_or(EngineError::UnknownScheduleItem)?;
                if item.notes != notes {
                    item.notes = notes;
                    self.schedule_changed = true;
                }
            }
            Command::PreviewScheduleItem { id } => {
                let item = self
                    .schedule
                    .get(id)
                    .ok_or(EngineError::UnknownScheduleItem)?;
                self.set_preview(self.pane_for(item, StartAt::First));
            }
            Command::ShowScheduleItem { id } => {
                let item = self
                    .schedule
                    .get(id)
                    .ok_or(EngineError::UnknownScheduleItem)?;
                let pane = self.pane_for(item, StartAt::First);
                self.go_live(pane);
                self.reveal();
            }
            Command::RefreshSong { song } => self.refresh_song(&song),
            Command::RefreshDeck { deck } => self.refresh_deck(&deck),

            // ── Slides inside a schedule item ─────────────────────────────
            Command::AppendSlides { id, slides } => {
                self.edit_deck(id, |deck| deck.slides.extend(slides))?;
            }
            Command::MoveItemSlide { id, from, to } => self.edit_deck(id, |deck| {
                if from < deck.slides.len() {
                    let slide = deck.slides.remove(from);
                    let to = to.min(deck.slides.len());
                    deck.slides.insert(to, slide);
                }
            })?,
            Command::RemoveItemSlide { id, index } => self.edit_deck(id, |deck| {
                // Keep at least one slide; removing the item is a separate step.
                if index < deck.slides.len() && deck.slides.len() > 1 {
                    deck.slides.remove(index);
                }
            })?,
            Command::SetItemAutoAdvance { id, auto_advance } => {
                self.edit_deck(id, |deck| deck.auto_advance = auto_advance)?;
            }
            Command::AutoAdvance {
                presentation_id,
                from,
            } => self.auto_advance(presentation_id, from),
        }
        Ok(())
    }

    /// Changes the deck of schedule item `id` (turning the item into one if
    /// it can be), then refreshes any pane showing it.
    fn edit_deck(
        &mut self,
        id: Id,
        change: impl FnOnce(&mut SlideDeck),
    ) -> Result<(), EngineError> {
        let item = self
            .schedule
            .get_mut(id)
            .ok_or(EngineError::UnknownScheduleItem)?;
        let before = item.content.clone();
        let deck = item.as_deck().ok_or(EngineError::CannotHoldSlides)?;
        change(deck);
        if item.content != before {
            self.schedule_changed = true;
            self.rebuild_panes_for(id);
        }
        Ok(())
    }

    /// Moves an auto-advancing live presentation on by one slide, starting
    /// again at the end if it loops. Does nothing if the operator already
    /// moved on, or the presentation is no longer live.
    fn auto_advance(&mut self, presentation_id: Id, from: usize) {
        let Some(pane) = self.live.as_mut() else {
            return;
        };
        let Some(settings) = pane.presentation.auto_advance else {
            return;
        };
        if pane.presentation.id != presentation_id || pane.selected != from {
            return;
        }
        if from + 1 < pane.presentation.slides.len() {
            pane.selected = from + 1;
        } else if settings.looping {
            pane.selected = 0;
        }
    }

    /// Applies a library edit of a deck to the schedule and to any pane
    /// showing it (see [`Engine::refresh_song`]).
    fn refresh_deck(&mut self, deck: &SlideDeck) {
        let changed_items = self.schedule.refresh_deck(deck);
        if !changed_items.is_empty() {
            self.schedule_changed = true;
        }
        for id in changed_items {
            self.rebuild_panes_for(id);
        }

        let look = self.look(ContentKind::Presentation, deck.theme_id);
        let presentation = Presentation::from_deck(deck, look, &self.themes);
        for pane in self.previews.values_mut().chain(self.live.as_mut()) {
            let shows_deck = matches!(
                pane.presentation.source,
                PresentationSource::Deck { deck_id } if deck_id == deck.id
            );
            if shows_deck && pane.schedule_item.is_none() {
                pane.presentation = presentation.clone();
                let last = pane.presentation.slides.len().saturating_sub(1);
                pane.selected = pane.selected.min(last);
            }
        }
    }

    /// The preview pane of the operator whose command this is.
    fn preview(&self) -> Option<&PaneState> {
        self.previews.get(&self.operator)
    }

    /// Puts `pane` in the preview of the operator whose command this is.
    fn set_preview(&mut self, pane: PaneState) {
        self.previews.insert(self.operator, pane);
    }

    fn live_mut(&mut self) -> Result<&mut PaneState, EngineError> {
        self.live.as_mut().ok_or(EngineError::NothingLive)
    }

    /// The look for content that is not in the schedule.
    fn look(&self, kind: ContentKind, own_theme: Option<Id>) -> crate::themes::Look {
        self.themes.resolve(kind, own_theme, &ItemLook::default())
    }

    /// Changes the live video's playback, if there is a live video.
    fn update_video(&mut self, change: impl FnOnce(&mut VideoPlayback, Timestamp)) {
        let now = (self.clock)();
        if let Some(video) = &mut self.video {
            change(video, now);
        }
    }

    /// Rebuilds any pane showing schedule item `id`, keeping its selected
    /// slide where possible.
    fn rebuild_panes_for(&mut self, id: Id) {
        let Some(item) = self.schedule.get(id) else {
            return;
        };
        let presentation = self.finish(item.presentation(&self.themes));
        for pane in self.previews.values_mut().chain(self.live.as_mut()) {
            if pane.schedule_item == Some(id) {
                let last = presentation.slides.len().saturating_sub(1);
                pane.presentation = presentation.clone();
                pane.selected = pane.selected.min(last);
            }
        }
    }

    /// Makes `pane` live. If it came from the schedule, the following item
    /// is loaded into preview so the operator can see what is next.
    ///
    /// A playlist item does not change the screen: "going live" with it
    /// starts the music, and what is on screen stays.
    fn go_live(&mut self, pane: PaneState) {
        let next = pane
            .schedule_item
            .and_then(|id| self.schedule.after(id))
            .map(|item| self.pane_for(item, StartAt::First));
        if let Some(next) = next {
            self.set_preview(next);
        }

        let playlist = pane
            .schedule_item
            .and_then(|id| self.schedule.get(id))
            .and_then(|item| match &item.content {
                ItemContent::Playlist { tracks, .. } => Some(tracks.clone()),
                _ => None,
            });
        match playlist {
            Some(tracks) => self.set_playlist(tracks, true),
            None => {
                self.start_countdown(&pane);
                self.live = Some(pane);
            }
        }
    }

    /// If `pane` shows a countdown, works out when it ends.
    fn start_countdown(&mut self, pane: &PaneState) {
        let countdown = pane
            .presentation
            .slides
            .get(pane.selected)
            .and_then(|slide| slide.countdown.as_ref());
        self.countdown_end = countdown.map(|countdown| {
            let ends_at = countdown
                .target
                .ends_at((self.clock)(), self.utc_offset_minutes);
            (pane.presentation.id, ends_at)
        });
    }

    /// A screen saw the live countdown reach zero. If it is set to move on,
    /// the next schedule item goes live. Several screens may report the same
    /// countdown, so this checks it is still live and really over.
    fn finish_countdown(&mut self) -> Result<(), EngineError> {
        let Some(pane) = &self.live else {
            return Ok(());
        };
        let moves_on = pane
            .presentation
            .slides
            .get(pane.selected)
            .and_then(|slide| slide.countdown.as_ref())
            .is_some_and(|countdown| countdown.at_end == CountdownEnd::NextItem);
        let over = self
            .countdown_end
            .is_some_and(|(id, ends_at)| id == pane.presentation.id && ends_at <= (self.clock)());
        if moves_on && over {
            self.countdown_end = None;
            self.next_slide()?;
        }
        Ok(())
    }

    /// Replaces the playlist with its audio tracks, optionally playing.
    fn set_playlist(&mut self, tracks: Vec<MediaItem>, play: bool) {
        self.audio.playlist = tracks
            .into_iter()
            .filter(|track| track.kind == MediaKind::Audio)
            .collect();
        self.audio.index = 0;
        self.audio.position_ms = 0;
        self.audio.playing = false;
        self.audio.fade = None;
        if play && !self.audio.playlist.is_empty() {
            self.audio.start_track(0, (self.clock)());
        }
    }

    /// Next track, or previous: back to the start of the current track if
    /// it has played for a few seconds, otherwise the track before.
    fn skip_track(&mut self, forward: bool) {
        const RESTART_AFTER_MS: u64 = 3_000;
        let now = (self.clock)();
        if self.audio.playlist.is_empty() {
            return;
        }
        let index = if forward {
            match self.audio.next_index() {
                Some(next) => next,
                None => return,
            }
        } else if self.audio.position_at(now) > RESTART_AFTER_MS {
            self.audio.index
        } else {
            self.audio.index.saturating_sub(1)
        };
        self.audio.start_track(index, now);
    }

    /// Adds the CCLI licence number to a song's footer, as the licence asks.
    fn finish(&self, presentation: Presentation) -> Presentation {
        license_footer(presentation, self.ccli_license.as_deref())
    }

    /// The "song went live" event, if a different song just went live.
    fn song_went_live(&self, before: Option<&PaneState>) -> Option<EngineEvent> {
        let live = self.live.as_ref()?;
        let PresentationSource::Song { song_id } = live.presentation.source else {
            return None;
        };
        let same_song = before.is_some_and(|pane| {
            matches!(pane.presentation.source, PresentationSource::Song { song_id: id } if id == song_id)
        });
        (!same_song).then(|| EngineEvent::SongWentLive {
            song_id,
            title: live.presentation.title.clone(),
        })
    }

    /// Advances one slide, continuing into the next schedule item at the end.
    fn next_slide(&mut self) -> Result<(), EngineError> {
        let pane = self.live_mut()?;
        if pane.selected + 1 < pane.presentation.slides.len() {
            pane.selected += 1;
            return Ok(());
        }

        let current = pane.schedule_item;
        if let Some(item) = current.and_then(|id| self.schedule.after(id)) {
            let next = self.pane_for(item, StartAt::First);
            self.go_live(next);
        }
        Ok(())
    }

    /// Goes back one slide, continuing into the previous schedule item at
    /// the start.
    fn previous_slide(&mut self) -> Result<(), EngineError> {
        let pane = self.live_mut()?;
        if pane.selected > 0 {
            pane.selected -= 1;
            return Ok(());
        }

        let current = pane.schedule_item;
        if let Some(item) = current.and_then(|id| self.schedule.before(id)) {
            let previous = self.pane_for(item, StartAt::Last);
            self.go_live(previous);
        }
        Ok(())
    }

    /// Builds a pane showing a schedule item.
    fn pane_for(&self, item: &ScheduleItem, start: StartAt) -> PaneState {
        let presentation = self.finish(item.presentation(&self.themes));
        let selected = match start {
            StartAt::First => 0,
            StartAt::Last => presentation.slides.len().saturating_sub(1),
        };
        PaneState {
            presentation,
            selected,
            schedule_item: Some(item.id),
        }
    }

    /// Applies a library edit to the schedule and to any pane showing the
    /// song, keeping the same slide selected (or the last one, if the song
    /// got shorter).
    fn refresh_song(&mut self, song: &Song) {
        let changed_items = self.schedule.refresh_song(song);
        if !changed_items.is_empty() {
            self.schedule_changed = true;
        }
        for id in changed_items {
            self.rebuild_panes_for(id);
        }

        // Panes showing the song straight from the library.
        let look = self.look(ContentKind::Song, song.theme_id);
        for pane in self.previews.values_mut().chain(self.live.as_mut()) {
            let shows_song = matches!(
                pane.presentation.source,
                PresentationSource::Song { song_id } if song_id == song.id
            );
            if shows_song && pane.schedule_item.is_none() {
                pane.presentation = license_footer(
                    Presentation::from_song(song, look.clone(), SlideOptions::default()),
                    self.ccli_license.as_deref(),
                );
                let last = pane.presentation.slides.len().saturating_sub(1);
                pane.selected = pane.selected.min(last);
            }
        }
    }

    /// Recomputes the output state. Returns it (with a new revision) if it
    /// differs from what renderers last received.
    fn refresh_output(&mut self) -> Option<OutputState> {
        let frame = self.live.as_ref().and_then(|pane| {
            let mut frame = pane.presentation.frame(pane.selected)?;
            frame.countdown = self.live_countdown(pane);
            Some(frame)
        });
        self.follow_live_video(frame.as_ref());
        let transition = self.live_transition();
        let stream = stream_output(
            &self.stream,
            &self.themes,
            self.live_kind(),
            frame.as_ref(),
            self.flags,
        );

        let unchanged = frame == self.output.frame
            && self.flags == self.output.flags
            && transition == self.output.transition
            && self.video == self.output.video
            && self.logo == self.output.logo
            && self.alert == self.output.alert
            && stream == self.output.stream;
        if unchanged {
            return None;
        }

        self.output = OutputState {
            frame,
            flags: self.flags,
            transition,
            video: self.video,
            logo: self.logo.clone(),
            alert: self.alert.clone(),
            stream,
            revision: self.output.revision + 1,
        };
        Some(self.output.clone())
    }

    /// The operator sent something live on purpose, so it should be seen:
    /// Logo and Clear Text come off. Black and Clear Background stay, as
    /// they are deliberate choices (and Black is the safety net). Moving on
    /// with Next does not do this, so a logo stays up between items.
    fn reveal(&mut self) {
        self.flags.logo = false;
        self.flags.hide_text = false;
    }

    /// What kind of content is live, for its transition and stream look.
    fn live_kind(&self) -> Option<StreamKind> {
        let source = &self.live.as_ref()?.presentation.source;
        Some(match source {
            PresentationSource::Song { .. } => StreamKind::Songs,
            PresentationSource::Scripture { .. } => StreamKind::Scripture,
            PresentationSource::Deck { .. } | PresentationSource::Slides => {
                StreamKind::Presentations
            }
            _ => StreamKind::Other,
        })
    }

    /// The transition for what is live: the one set for its kind of content.
    fn live_transition(&self) -> Transition {
        let settings = &self.transitions;
        match self.live_kind() {
            Some(StreamKind::Songs) => settings.songs,
            Some(StreamKind::Scripture) => settings.scripture,
            Some(StreamKind::Presentations) => settings.presentations,
            Some(StreamKind::Other) | None => settings.other,
        }
    }

    /// The countdown to draw for the live pane, if it shows one.
    fn live_countdown(&self, pane: &PaneState) -> Option<CountdownFrame> {
        let countdown = pane
            .presentation
            .slides
            .get(pane.selected)?
            .countdown
            .as_ref()?;
        let (id, ends_at) = self.countdown_end?;
        (id == pane.presentation.id).then(|| CountdownFrame {
            message: countdown.message.clone(),
            ends_at,
        })
    }

    /// Keeps playback in step with the live frame: a new video starts from
    /// the beginning, the same video carries on, no video means no playback.
    fn follow_live_video(&mut self, frame: Option<&RenderFrame>) {
        let live_video = frame.and_then(|frame| match &frame.theme.background {
            Background::Video {
                media,
                looping,
                play_audio,
                ..
            } => Some((media.id, *looping, *play_audio)),
            _ => None,
        });

        self.video = match (live_video, self.video) {
            (Some((id, ..)), Some(current)) if current.media_id == id => Some(current),
            (Some((id, looping, play_audio)), _) => Some(VideoPlayback {
                media_id: id,
                playing: true,
                position_ms: 0,
                anchor: (self.clock)(),
                looping,
                muted: !play_audio,
            }),
            (None, _) => None,
        };
    }
}

/// Selects `index` in a pane, checking it is in range.
fn select(pane: &mut PaneState, index: usize) -> Result<(), EngineError> {
    let count = pane.presentation.slides.len();
    if index >= count {
        return Err(EngineError::SlideOutOfRange { index, count });
    }
    pane.selected = index;
    Ok(())
}

/// Adds "CCLI License # …" to a song's footer. Other presentations are
/// returned unchanged. (A free function, for places that cannot borrow the
/// whole engine.)
fn license_footer(mut presentation: Presentation, license: Option<&str>) -> Presentation {
    let is_song = matches!(presentation.source, PresentationSource::Song { .. });
    if let (true, Some(license)) = (is_song, license) {
        let notice = format!("CCLI License # {license}");
        presentation.footer = Some(match presentation.footer.take() {
            Some(footer) => format!("{footer} · {notice}"),
            None => notice,
        });
    }
    presentation
}

/// A pane for a presentation that did not come from the schedule.
fn unscheduled(presentation: Presentation) -> PaneState {
    PaneState {
        presentation,
        selected: 0,
        schedule_item: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::{AutoAdvance, DeckSlide};
    use crate::schedule::ItemContent;
    use crate::theme::Theme;

    fn three_slides() -> Presentation {
        Presentation::from_text("Test", "one\n\ntwo\n\nthree", Theme::default())
    }

    /// An engine with `three_slides()` in preview and live, on slide 0.
    fn live_engine() -> Engine {
        let mut engine = Engine::new();
        engine
            .dispatch(Command::ShowInPreview {
                presentation: Box::new(three_slides()),
            })
            .unwrap();
        engine.dispatch(Command::GoLive).unwrap();
        engine
    }

    fn live_text(engine: &Engine) -> Option<String> {
        engine.snapshot().output.frame.map(|f| f.text)
    }

    fn preview_title(engine: &Engine) -> Option<String> {
        engine.snapshot().preview.map(|p| p.presentation.title)
    }

    fn text_item(title: &str, text: &str) -> ScheduleItem {
        ScheduleItem::new(ItemContent::Text {
            title: title.into(),
            text: text.into(),
        })
    }

    /// An engine with a three-item schedule: "A" (slides a1, a2),
    /// "B" (b1) and "C" (c1, c2). Returns the item ids too.
    fn scheduled_engine() -> (Engine, [Id; 3]) {
        let items = [
            text_item("A", "a1\n\na2"),
            text_item("B", "b1"),
            text_item("C", "c1\n\nc2"),
        ];
        let ids = [items[0].id, items[1].id, items[2].id];

        let mut engine = Engine::new();
        engine
            .dispatch(Command::ReplaceSchedule {
                schedule: Box::new(Schedule {
                    items: items.into(),
                }),
            })
            .unwrap();
        (engine, ids)
    }

    // ── Preview, live and display controls ───────────────────────────────

    #[test]
    fn preview_does_not_touch_the_output() {
        let mut engine = Engine::new();
        let events = engine
            .dispatch(Command::ShowInPreview {
                presentation: Box::new(three_slides()),
            })
            .unwrap();

        assert!(matches!(
            events.as_slice(),
            [EngineEvent::PreviewChanged { .. }]
        ));
        assert_eq!(engine.snapshot().output.frame, None);
    }

    #[test]
    fn go_live_shows_the_selected_preview_slide() {
        let mut engine = Engine::new();
        engine
            .dispatch(Command::ShowInPreview {
                presentation: Box::new(three_slides()),
            })
            .unwrap();
        engine
            .dispatch(Command::SelectPreviewSlide { index: 1 })
            .unwrap();

        let events = engine.dispatch(Command::GoLive).unwrap();
        assert!(matches!(
            events.as_slice(),
            [
                EngineEvent::LiveChanged { .. },
                EngineEvent::OutputChanged { .. }
            ]
        ));
        assert_eq!(live_text(&engine).as_deref(), Some("two"));
    }

    #[test]
    fn next_and_previous_stop_at_the_ends_outside_a_schedule() {
        let mut engine = live_engine();

        assert!(engine.dispatch(Command::PreviousSlide).unwrap().is_empty());

        engine.dispatch(Command::NextSlide).unwrap();
        engine.dispatch(Command::NextSlide).unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("three"));

        assert!(engine.dispatch(Command::NextSlide).unwrap().is_empty());
    }

    #[test]
    fn flags_change_the_output_but_not_the_live_slide() {
        let mut engine = live_engine();
        let revision = engine.snapshot().output.revision;

        let events = engine.dispatch(Command::SetBlack { on: true }).unwrap();
        assert!(matches!(
            events.as_slice(),
            [EngineEvent::OutputChanged { .. }]
        ));

        let output = engine.snapshot().output;
        assert!(output.flags.black);
        assert_eq!(output.revision, revision + 1);
        assert_eq!(live_text(&engine).as_deref(), Some("one"));

        // Setting the same value again is a no-op.
        assert!(
            engine
                .dispatch(Command::SetBlack { on: true })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn errors_for_missing_panes_and_bad_indexes() {
        let mut engine = Engine::new();
        assert_eq!(
            engine.dispatch(Command::GoLive),
            Err(EngineError::NothingInPreview)
        );
        assert_eq!(
            engine.dispatch(Command::NextSlide),
            Err(EngineError::NothingLive)
        );
        assert_eq!(
            engine.dispatch(Command::ShowScheduleItem { id: Id::new() }),
            Err(EngineError::UnknownScheduleItem)
        );

        let mut engine = live_engine();
        assert_eq!(
            engine.dispatch(Command::SelectLiveSlide { index: 3 }),
            Err(EngineError::SlideOutOfRange { index: 3, count: 3 })
        );
    }

    #[test]
    fn clear_live_blanks_the_output() {
        let mut engine = live_engine();
        engine.dispatch(Command::ClearLive).unwrap();
        assert_eq!(engine.snapshot().live, None);
        assert_eq!(live_text(&engine), None);
    }

    #[test]
    fn commands_round_trip_through_json() {
        let json = r#"{"type":"jumpToSection","shortcut":"C"}"#;
        let command: Command = serde_json::from_str(json).unwrap();
        assert_eq!(
            command,
            Command::JumpToSection {
                shortcut: "C".into()
            }
        );
        assert_eq!(serde_json::to_string(&command).unwrap(), json);

        let json =
            r#"{"type":"moveScheduleItem","id":"01920c7e-5a4b-7c3d-9e8f-0a1b2c3d4e5f","to":2}"#;
        let command: Command = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&command).unwrap(), json);
    }

    // ── Running a service from the schedule ──────────────────────────────

    #[test]
    fn schedule_edits_announce_the_new_schedule() {
        let (mut engine, [a, _, c]) = scheduled_engine();

        let events = engine
            .dispatch(Command::MoveScheduleItem { id: c, to: 0 })
            .unwrap();
        assert!(matches!(
            events.as_slice(),
            [EngineEvent::ScheduleChanged { .. }]
        ));
        assert_eq!(engine.schedule().items[0].id, c);

        let events = engine
            .dispatch(Command::SetScheduleItemNotes {
                id: a,
                notes: "Pray first".into(),
            })
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(engine.schedule().get(a).unwrap().notes, "Pray first");

        // Removing something that is already gone is not an error.
        engine
            .dispatch(Command::RemoveScheduleItem { id: a })
            .unwrap();
        let events = engine
            .dispatch(Command::RemoveScheduleItem { id: a })
            .unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn going_live_previews_the_next_item() {
        let (mut engine, [a, b, _]) = scheduled_engine();

        engine
            .dispatch(Command::ShowScheduleItem { id: a })
            .unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("a1"));
        assert_eq!(preview_title(&engine).as_deref(), Some("B"));
        assert_eq!(engine.snapshot().preview.unwrap().schedule_item, Some(b));

        // Sending the preview live moves the preview on again.
        engine.dispatch(Command::GoLive).unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("b1"));
        assert_eq!(preview_title(&engine).as_deref(), Some("C"));
    }

    #[test]
    fn next_and_previous_continue_across_items() {
        let (mut engine, [a, _, _]) = scheduled_engine();
        engine
            .dispatch(Command::ShowScheduleItem { id: a })
            .unwrap();

        let mut seen = vec![live_text(&engine).unwrap()];
        for _ in 0..5 {
            engine.dispatch(Command::NextSlide).unwrap();
            seen.push(live_text(&engine).unwrap());
        }
        assert_eq!(seen, ["a1", "a2", "b1", "c1", "c2", "c2"]);

        let mut seen = Vec::new();
        for _ in 0..5 {
            engine.dispatch(Command::PreviousSlide).unwrap();
            seen.push(live_text(&engine).unwrap());
        }
        assert_eq!(seen, ["c1", "b1", "a2", "a1", "a1"]);
    }

    #[test]
    fn the_last_item_keeps_the_preview_it_had() {
        let (mut engine, [_, _, c]) = scheduled_engine();
        engine
            .dispatch(Command::PreviewScheduleItem { id: c })
            .unwrap();
        engine.dispatch(Command::GoLive).unwrap();

        // There is nothing after C, so the preview still shows C.
        assert_eq!(preview_title(&engine).as_deref(), Some("C"));
    }

    #[test]
    fn removing_the_live_item_keeps_it_on_screen() {
        let (mut engine, [a, _, _]) = scheduled_engine();
        engine
            .dispatch(Command::ShowScheduleItem { id: a })
            .unwrap();
        engine
            .dispatch(Command::RemoveScheduleItem { id: a })
            .unwrap();

        assert_eq!(live_text(&engine).as_deref(), Some("a1"));
        engine.dispatch(Command::NextSlide).unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("a2"));

        // At the end there is no known "next item" any more: it just stops.
        engine.dispatch(Command::NextSlide).unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("a2"));
    }

    #[test]
    fn refresh_song_updates_schedule_and_live_slides() {
        let mut song = Song::new("Song");
        song.lyrics = "[V1]\nfirst\n\nsecond".into();
        let item = ScheduleItem::new(ItemContent::Song { song: song.clone() });
        let id = item.id;

        let mut engine = Engine::new();
        engine
            .dispatch(Command::AddScheduleItem {
                item: Box::new(item),
                index: None,
            })
            .unwrap();
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();
        engine.dispatch(Command::NextSlide).unwrap();

        // Fix a typo and make the song shorter.
        song.lyrics = "[V1]\nfirst (fixed)".into();
        let events = engine
            .dispatch(Command::RefreshSong {
                song: Box::new(song),
            })
            .unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::ScheduleChanged { .. }))
        );
        assert_eq!(live_text(&engine).as_deref(), Some("first (fixed)"));
        assert_eq!(engine.snapshot().live.unwrap().selected, 0);
    }

    // ── Themes, media and video ──────────────────────────────────────────

    use crate::media::{MediaItem, MediaKind};
    use crate::theme::MediaFit;
    use crate::themes::{ThemeRecord, starter_library};
    use std::cell::Cell;

    thread_local! {
        /// The fake time for video tests. Thread-local, so tests running in
        /// parallel do not disturb each other.
        static NOW: Cell<Timestamp> = const { Cell::new(1_000) };
    }

    fn fake_now() -> Timestamp {
        NOW.with(Cell::get)
    }

    fn advance(ms: Timestamp) {
        NOW.with(|now| now.set(now.get() + ms));
    }

    fn video(name: &str) -> MediaItem {
        MediaItem {
            id: Id::new(),
            kind: MediaKind::Video,
            name: name.into(),
            file: format!("{name}.mp4"),
            width: None,
            height: None,
            duration_ms: Some(60_000),
            size_bytes: 1,
            created_at: 0,
            collection: None,
        }
    }

    fn video_background(media: &MediaItem) -> Background {
        Background::Video {
            media: media.as_ref(),
            fit: MediaFit::Cover,
            looping: true,
            play_audio: false,
            opacity: 1.0,
        }
    }

    /// An engine with the starter themes and the given schedule.
    fn themed_engine(items: Vec<ScheduleItem>) -> Engine {
        let mut engine = Engine::with_clock(fake_now);
        engine
            .dispatch(Command::SetThemes {
                library: Box::new(starter_library()),
            })
            .unwrap();
        engine
            .dispatch(Command::ReplaceSchedule {
                schedule: Box::new(Schedule { items }),
            })
            .unwrap();
        engine
    }

    fn live_theme_name(engine: &Engine) -> String {
        engine.snapshot().output.frame.unwrap().theme.name
    }

    #[test]
    fn previews_use_the_default_theme_for_their_kind() {
        let mut engine = themed_engine(Vec::new());
        engine
            .dispatch(Command::PreviewText {
                title: "Welcome".into(),
                text: "Hello".into(),
            })
            .unwrap();
        let preview = engine.snapshot().preview.unwrap();
        assert_eq!(preview.presentation.theme.name, "Default");
        assert!(preview.presentation.theme_id.is_some());
    }

    #[test]
    fn editing_a_theme_restyles_what_is_live() {
        let item = text_item("A", "a1");
        let id = item.id;
        let mut engine = themed_engine(vec![item]);
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();

        let mut library = starter_library();
        library.themes[0].theme.text.color = "#ff0000".into();
        let events = engine
            .dispatch(Command::SetThemes {
                library: Box::new(library),
            })
            .unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::OutputChanged { .. }))
        );
        let frame = engine.snapshot().output.frame.unwrap();
        assert_eq!(frame.theme.text.color, "#ff0000");
    }

    #[test]
    fn an_item_look_changes_that_item_only() {
        let first = text_item("A", "a1");
        let second = text_item("B", "b1");
        let (a, b) = (first.id, second.id);
        let mut engine = themed_engine(vec![first, second]);
        engine
            .dispatch(Command::ShowScheduleItem { id: a })
            .unwrap();

        let clean: ThemeRecord = starter_library().themes[1].clone();
        engine
            .dispatch(Command::SetScheduleItemLook {
                id: a,
                look: Box::new(ItemLook {
                    theme: Some(clean),
                    background: None,
                }),
            })
            .unwrap();
        assert_eq!(
            live_theme_name(&engine),
            "Clean",
            "the live item is redrawn"
        );

        engine
            .dispatch(Command::ShowScheduleItem { id: b })
            .unwrap();
        assert_eq!(live_theme_name(&engine), "Default");
    }

    #[test]
    fn a_video_keeps_playing_across_slides_that_share_it() {
        let clip = video("loop");
        let mut item = text_item("Song", "one\n\ntwo");
        item.look.background = Some(video_background(&clip));
        let id = item.id;
        let mut engine = themed_engine(vec![item]);

        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();
        let started = engine.snapshot().output.video.unwrap();
        assert!(started.playing);
        assert!(started.muted, "a background without sound starts muted");
        assert_eq!(started.media_id, clip.id);

        advance(5_000);
        engine.dispatch(Command::NextSlide).unwrap();
        let after = engine.snapshot().output.video.unwrap();
        assert_eq!(after, started, "same video, same clock: no restart");
        assert_eq!(after.position_at(fake_now()), 5_000);
    }

    #[test]
    fn pause_seek_and_play_move_the_shared_clock() {
        let clip = video("clip");
        let item = ScheduleItem::new(crate::schedule::ItemContent::Media { media: clip });
        let id = item.id;
        let mut engine = themed_engine(vec![item]);
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();

        advance(2_000);
        engine.dispatch(Command::PauseVideo).unwrap();
        let paused = engine.snapshot().output.video.unwrap();
        assert!(!paused.playing);
        advance(10_000);
        assert_eq!(
            paused.position_at(fake_now()),
            2_000,
            "paused video stays put"
        );

        engine
            .dispatch(Command::SeekVideo {
                position_ms: 30_000,
            })
            .unwrap();
        engine.dispatch(Command::PlayVideo).unwrap();
        advance(1_000);
        let playing = engine.snapshot().output.video.unwrap();
        assert_eq!(playing.position_at(fake_now()), 31_000);
        assert!(!playing.looping, "media items do not loop by default");
        assert!(!playing.muted, "media items play their sound by default");

        engine
            .dispatch(Command::SetVideoMuted { on: true })
            .unwrap();
        let muted = engine.snapshot().output.video.unwrap();
        assert!(muted.muted);
        assert_eq!(
            muted.position_at(fake_now()),
            31_000,
            "muting does not move the video"
        );
    }

    #[test]
    fn a_different_video_starts_from_the_beginning() {
        let (first, second) = (video("first"), video("second"));
        let a = ScheduleItem::new(crate::schedule::ItemContent::Media { media: first });
        let b = ScheduleItem::new(crate::schedule::ItemContent::Media {
            media: second.clone(),
        });
        let (a_id, b_id) = (a.id, b.id);
        let mut engine = themed_engine(vec![a, b]);

        engine
            .dispatch(Command::ShowScheduleItem { id: a_id })
            .unwrap();
        advance(4_000);
        engine
            .dispatch(Command::ShowScheduleItem { id: b_id })
            .unwrap();

        let playback = engine.snapshot().output.video.unwrap();
        assert_eq!(playback.media_id, second.id);
        assert_eq!(playback.position_at(fake_now()), 0);

        engine.dispatch(Command::ClearLive).unwrap();
        assert_eq!(engine.snapshot().output.video, None);
    }

    // ── Alerts, countdowns, stage, audio, CCLI ───────────────────────────

    use crate::alerts::{AlertEnd, AlertPosition, AlertRequest, AlertStyle};
    use crate::countdown::{CountdownSlide, CountdownTarget};

    fn track(name: &str) -> MediaItem {
        MediaItem {
            kind: MediaKind::Audio,
            ..video(name)
        }
    }

    #[test]
    fn alerts_show_and_clear() {
        let mut engine = Engine::with_clock(fake_now);
        engine
            .dispatch(Command::ShowAlert {
                alert: AlertRequest {
                    text: "Parent of child #4".into(),
                    style: AlertStyle::Ticker,
                    position: AlertPosition::Bottom,
                    end: AlertEnd::AfterPasses { passes: 3 },
                },
            })
            .unwrap();
        let alert = engine.snapshot().output.alert.unwrap();
        assert_eq!(alert.request.text, "Parent of child #4");
        assert_eq!(alert.started_at, fake_now());

        engine.dispatch(Command::ClearAlert).unwrap();
        assert_eq!(engine.snapshot().output.alert, None);
    }

    #[test]
    fn a_countdown_ends_and_moves_on_when_asked() {
        let countdown = ScheduleItem::new(ItemContent::Countdown {
            title: "Countdown".into(),
            countdown: CountdownSlide {
                message: "Service starts in".into(),
                target: CountdownTarget::Duration { seconds: 60 },
                at_end: CountdownEnd::NextItem,
            },
        });
        let song = text_item("First song", "line");
        let id = countdown.id;
        let mut engine = themed_engine(vec![countdown, song]);
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();

        let frame = engine.snapshot().output.frame.unwrap();
        assert_eq!(frame.countdown.unwrap().ends_at, fake_now() + 60_000);

        // Reported too early (a screen's clock is ahead): nothing happens.
        engine.dispatch(Command::CountdownFinished).unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("Service starts in"));

        advance(60_000);
        engine.dispatch(Command::CountdownFinished).unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("line"));
    }

    #[test]
    fn the_stage_timer_pauses_and_resumes() {
        let mut engine = Engine::with_clock(fake_now);
        engine
            .dispatch(Command::StartStageTimer {
                mode: TimerMode::CountDown,
                duration_ms: 30 * 60_000,
            })
            .unwrap();
        advance(10_000);
        engine.dispatch(Command::PauseStageTimer).unwrap();
        advance(50_000);
        engine.dispatch(Command::ResumeStageTimer).unwrap();
        advance(5_000);

        let timer = engine.snapshot().stage.timer.unwrap();
        assert_eq!(
            timer.elapsed_at(fake_now()),
            15_000,
            "the pause does not count"
        );

        let events = engine
            .dispatch(Command::SetStageMessage {
                message: Some("Wrap up".into()),
            })
            .unwrap();
        assert!(matches!(
            events.as_slice(),
            [EngineEvent::StageChanged { .. }]
        ));
    }

    #[test]
    fn audio_plays_skips_and_ends() {
        let mut engine = Engine::with_clock(fake_now);
        engine
            .dispatch(Command::SetPlaylist {
                tracks: vec![track("one"), video("not audio"), track("two")],
                play: true,
            })
            .unwrap();
        let audio = engine.snapshot().audio;
        assert_eq!(audio.playlist.len(), 2, "only audio is kept");
        assert!(audio.playing);

        engine.dispatch(Command::TrackEnded).unwrap();
        assert_eq!(engine.snapshot().audio.index, 1);

        engine.dispatch(Command::TrackEnded).unwrap();
        assert!(
            !engine.snapshot().audio.playing,
            "stops after the last track"
        );

        engine.dispatch(Command::SetAudioLoop { on: true }).unwrap();
        engine.dispatch(Command::PlayTrack { index: 1 }).unwrap();
        engine.dispatch(Command::TrackEnded).unwrap();
        assert_eq!(engine.snapshot().audio.index, 0, "loops back to the start");
    }

    #[test]
    fn a_playlist_item_plays_without_changing_the_screen() {
        let words = text_item("Welcome", "Welcome!");
        let playlist = ScheduleItem::new(ItemContent::Playlist {
            name: "Before service".into(),
            tracks: vec![track("one")],
        });
        let (words_id, playlist_id) = (words.id, playlist.id);
        let mut engine = themed_engine(vec![words, playlist]);
        engine
            .dispatch(Command::ShowScheduleItem { id: words_id })
            .unwrap();
        engine
            .dispatch(Command::ShowScheduleItem { id: playlist_id })
            .unwrap();

        assert_eq!(live_text(&engine).as_deref(), Some("Welcome!"));
        assert!(engine.snapshot().audio.playing);
    }

    #[test]
    fn songs_going_live_are_reported_once_and_carry_the_licence() {
        let mut song = Song::new("Amazing Grace");
        song.lyrics = "[V1]\none\n\ntwo".into();
        let mut engine = themed_engine(Vec::new());
        engine
            .dispatch(Command::SetCcliLicense {
                license: Some("12345".into()),
            })
            .unwrap();
        engine
            .dispatch(Command::PreviewSong {
                song: Box::new(song.clone()),
            })
            .unwrap();

        let events = engine.dispatch(Command::GoLive).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::SongWentLive { .. }))
        );
        let footer = engine.snapshot().output.frame.unwrap().footer.unwrap();
        assert!(footer.ends_with("CCLI License # 12345"), "{footer}");

        let events = engine.dispatch(Command::NextSlide).unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EngineEvent::SongWentLive { .. }))
        );
    }

    // ── Slides inside a schedule item ────────────────────────────────────

    fn deck_item(texts: &[&str]) -> ScheduleItem {
        let mut deck = SlideDeck::new("Notices");
        deck.slides = texts.iter().map(|text| DeckSlide::text(*text)).collect();
        ScheduleItem::new(ItemContent::Deck { deck })
    }

    fn item_slide_count(engine: &Engine, id: Id) -> usize {
        engine
            .schedule()
            .get(id)
            .unwrap()
            .presentation(engine.themes())
            .slides
            .len()
    }

    #[test]
    fn pictures_dropped_on_a_picture_make_a_slideshow() {
        let item = ScheduleItem::new(ItemContent::Media {
            media: video("Welcome"),
        });
        let id = item.id;
        let mut engine = themed_engine(vec![item]);

        let slides = vec![DeckSlide::picture(&video("Notices")).unwrap()];
        let events = engine
            .dispatch(Command::AppendSlides { id, slides })
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::ScheduleChanged { .. }))
        );

        let content = &engine.schedule().get(id).unwrap().content;
        assert!(matches!(content, ItemContent::Deck { deck } if deck.name == "Welcome"));
        assert_eq!(item_slide_count(&engine, id), 2);
    }

    #[test]
    fn songs_cannot_take_extra_slides() {
        let item = ScheduleItem::new(ItemContent::Song {
            song: Song::new("Song"),
        });
        let id = item.id;
        let mut engine = themed_engine(vec![item]);
        let result = engine.dispatch(Command::AppendSlides {
            id,
            slides: vec![DeckSlide::text("x")],
        });
        assert_eq!(result, Err(EngineError::CannotHoldSlides));
    }

    #[test]
    fn slides_in_an_item_can_be_reordered_and_removed() {
        let item = deck_item(&["a", "b", "c"]);
        let id = item.id;
        let mut engine = themed_engine(vec![item]);
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();

        engine
            .dispatch(Command::MoveItemSlide { id, from: 2, to: 0 })
            .unwrap();
        engine
            .dispatch(Command::RemoveItemSlide { id, index: 1 })
            .unwrap();

        let texts: Vec<_> = engine
            .snapshot()
            .live
            .unwrap()
            .presentation
            .slides
            .iter()
            .map(|slide| slide.text.clone())
            .collect();
        assert_eq!(texts, ["c", "b"], "the live pane follows the change");
    }

    #[test]
    fn auto_advance_moves_on_and_loops_but_ignores_stale_timers() {
        let item = deck_item(&["a", "b"]);
        let id = item.id;
        let mut engine = themed_engine(vec![item]);
        engine
            .dispatch(Command::SetItemAutoAdvance {
                id,
                auto_advance: Some(AutoAdvance {
                    seconds: 8,
                    looping: true,
                }),
            })
            .unwrap();
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();
        let presentation_id = engine.snapshot().live.unwrap().presentation.id;

        engine
            .dispatch(Command::AutoAdvance {
                presentation_id,
                from: 0,
            })
            .unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("b"));

        // A timer for a slide the operator already left does nothing.
        engine
            .dispatch(Command::AutoAdvance {
                presentation_id,
                from: 0,
            })
            .unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("b"));

        engine
            .dispatch(Command::AutoAdvance {
                presentation_id,
                from: 1,
            })
            .unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("a"), "loops back");
    }

    #[test]
    fn library_edits_of_a_deck_reach_the_schedule() {
        let item = deck_item(&["old"]);
        let id = item.id;
        let ItemContent::Deck { deck } = &item.content else {
            unreachable!()
        };
        let mut edited = deck.clone();
        edited.slides[0].text = "new".into();
        let mut engine = themed_engine(vec![item]);
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();

        engine
            .dispatch(Command::RefreshDeck {
                deck: Box::new(edited),
            })
            .unwrap();
        assert_eq!(live_text(&engine).as_deref(), Some("new"));
    }

    #[test]
    fn sending_something_live_takes_off_logo_and_clear_text_but_not_black() {
        let mut engine = live_engine();
        for command in [
            Command::SetLogo { on: true },
            Command::SetHideText { on: true },
            Command::SetBlack { on: true },
        ] {
            engine.dispatch(command).unwrap();
        }
        engine
            .dispatch(Command::ShowInPreview {
                presentation: Box::new(three_slides()),
            })
            .unwrap();
        engine.dispatch(Command::GoLive).unwrap();

        let flags = engine.snapshot().output.flags;
        assert!(!flags.logo && !flags.hide_text);
        assert!(flags.black, "Black stays until the operator turns it off");
    }

    #[test]
    fn next_into_the_following_item_keeps_the_logo() {
        let (mut engine, [a, _, _]) = scheduled_engine();
        engine
            .dispatch(Command::ShowScheduleItem { id: a })
            .unwrap();
        engine.dispatch(Command::SetLogo { on: true }).unwrap();
        for _ in 0..3 {
            engine.dispatch(Command::NextSlide).unwrap();
        }
        assert!(engine.snapshot().output.flags.logo);
    }

    #[test]
    fn each_operator_has_their_own_preview_and_shares_live() {
        let mut engine = themed_engine(Vec::new());
        let pulpit = Id::new();
        let text = |title: &str| Command::PreviewText {
            title: title.into(),
            text: title.into(),
        };

        engine.dispatch(text("Welcome")).unwrap();
        let events = engine.dispatch_as(pulpit, text("John 3:16")).unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            EngineEvent::PreviewChanged { operator, .. } if *operator == pulpit
        )));

        // Neither disturbed the other's preview.
        let title = |snapshot: EngineSnapshot| snapshot.preview.map(|pane| pane.presentation.title);
        assert_eq!(title(engine.snapshot()), Some("Welcome".to_owned()));
        assert_eq!(
            title(engine.snapshot_for(pulpit)),
            Some("John 3:16".to_owned())
        );

        // Go Live sends the sender's own preview; everyone sees it live.
        engine.dispatch_as(pulpit, Command::GoLive).unwrap();
        let live = engine.snapshot().live.unwrap().presentation.title;
        assert_eq!(live, "John 3:16");
        assert_eq!(title(engine.snapshot()), Some("Welcome".to_owned()));
    }

    #[test]
    fn a_camera_goes_live_full_screen_but_not_on_the_stream() {
        let mut engine = themed_engine(Vec::new());
        let camera = crate::media::CameraRef {
            device_id: "cam-1".into(),
            label: "Elgato HD60 S".into(),
        };
        engine
            .dispatch(Command::PreviewCamera {
                camera: camera.clone(),
            })
            .unwrap();
        assert_eq!(
            engine.snapshot().preview.unwrap().presentation.title,
            "Elgato HD60 S"
        );
        engine.dispatch(Command::GoLive).unwrap();

        let output = engine.snapshot().output;
        let frame = output.frame.expect("the camera is live");
        assert!(matches!(
            frame.theme.background,
            Background::Camera { camera: ref live, .. } if *live == camera
        ));
        // The stream's switcher has the camera itself.
        let stream = output.stream.frame.expect("the stream shows the slide");
        assert!(!matches!(
            stream.theme.background,
            Background::Camera { .. }
        ));
    }

    #[test]
    fn the_stream_shows_a_live_song_as_a_lower_third() {
        let mut song = Song::new("Song");
        song.lyrics = "[V1]\nfirst\n\nsecond".into();
        let item = ScheduleItem::new(ItemContent::Song { song });
        let id = item.id;
        let mut engine = themed_engine(vec![item]);
        engine.dispatch(Command::ShowScheduleItem { id }).unwrap();

        let output = engine.snapshot().output;
        assert_eq!(output.frame.as_ref().unwrap().theme.name, "Default");
        let stream = output.stream.frame.expect("the song is on the stream");
        assert_eq!(stream.text, "first");
        assert_eq!(stream.theme.name, "Lower Third");

        // Black clears the stream rather than covering the camera.
        engine.dispatch(Command::SetBlack { on: true }).unwrap();
        assert_eq!(engine.snapshot().output.stream.frame, None);

        // Changing the stream look takes effect straight away.
        engine.dispatch(Command::SetBlack { on: false }).unwrap();
        let events = engine
            .dispatch(Command::SetStreamSettings {
                settings: StreamSettings {
                    songs: crate::stream::StreamLook::SameAsMain,
                    ..StreamSettings::default()
                },
            })
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::StreamSettingsChanged { .. }))
        );
        let stream = engine.snapshot().output.stream.frame.unwrap();
        assert_eq!(stream.theme.name, "Default");
    }

    #[test]
    fn each_kind_of_content_has_its_own_transition() {
        let cut = Transition {
            kind: crate::output::TransitionKind::Cut,
            duration_ms: 0,
        };
        let deck = deck_item(&["Notices"]);
        let deck_id = deck.id;
        let mut engine = themed_engine(vec![deck]);
        engine
            .dispatch(Command::SetTransitions {
                transitions: TransitionSettings {
                    presentations: cut,
                    ..TransitionSettings::default()
                },
            })
            .unwrap();

        // Typed text is "everything else": still a fade.
        engine
            .dispatch(Command::ShowInPreview {
                presentation: Box::new(three_slides()),
            })
            .unwrap();
        engine.dispatch(Command::GoLive).unwrap();
        assert_eq!(engine.snapshot().output.transition, Transition::default());

        // A presentation cuts straight in.
        engine
            .dispatch(Command::ShowScheduleItem { id: deck_id })
            .unwrap();
        assert_eq!(engine.snapshot().output.transition, cut);
    }
}
