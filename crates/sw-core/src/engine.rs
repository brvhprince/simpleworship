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

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::command::{Command, EngineEvent};
use crate::id::Id;
use crate::media::MediaRef;
use crate::output::{DisplayFlags, OutputState, RenderFrame, Transition, VideoPlayback};
use crate::presentation::{Presentation, PresentationSource};
use crate::schedule::{Schedule, ScheduleItem};
use crate::slides::SlideOptions;
use crate::song::Song;
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
}

/// The presentation engine. See the [module docs](self).
#[derive(Debug)]
pub struct Engine {
    schedule: Schedule,
    preview: Option<PaneState>,
    live: Option<PaneState>,
    flags: DisplayFlags,
    transition: Transition,
    /// The themes and defaults everything is styled with.
    themes: ThemeLibrary,
    /// The church logo, if one is set.
    logo: Option<MediaRef>,
    /// Playback of the live video background, if there is one.
    video: Option<VideoPlayback>,
    /// Where the engine gets the time, for video timing.
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
            preview: None,
            live: None,
            flags: DisplayFlags::default(),
            transition: Transition::default(),
            themes: ThemeLibrary::default(),
            logo: None,
            video: None,
            clock,
            schedule_changed: false,
            output: OutputState::default(),
        }
    }

    /// A full copy of the current state.
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            schedule: self.schedule.clone(),
            themes: self.themes.clone(),
            preview: self.preview.clone(),
            live: self.live.clone(),
            output: self.output.clone(),
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

    /// Applies a command and returns the events describing what changed.
    ///
    /// Commands that change nothing (pressing "next" on the last slide,
    /// turning black on when it is already on) return no events.
    pub fn dispatch(&mut self, command: Command) -> Result<Vec<EngineEvent>, EngineError> {
        let before_preview = self.preview.clone();
        let before_live = self.live.clone();
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
        if self.preview != before_preview {
            events.push(EngineEvent::PreviewChanged {
                preview: self.preview.clone(),
            });
        }
        if self.live != before_live {
            events.push(EngineEvent::LiveChanged {
                live: self.live.clone(),
            });
        }
        if let Some(output) = self.refresh_output() {
            events.push(EngineEvent::OutputChanged { output });
        }
        Ok(events)
    }

    /// Mutates the state for one command. Event bookkeeping is done by
    /// [`Engine::dispatch`].
    fn apply(&mut self, command: Command) -> Result<(), EngineError> {
        match command {
            // ── Preview and live ──────────────────────────────────────────
            Command::ShowInPreview { presentation } => {
                self.preview = Some(unscheduled(*presentation));
            }
            Command::PreviewSong { song } => {
                let look = self.look(ContentKind::Song, song.theme_id);
                let presentation = Presentation::from_song(&song, look, SlideOptions::default());
                self.preview = Some(unscheduled(presentation));
            }
            Command::PreviewText { title, text } => {
                let look = self.look(ContentKind::Text, None);
                self.preview = Some(unscheduled(Presentation::from_text(title, &text, look)));
            }
            Command::PreviewScripture {
                passage,
                translations,
                options,
            } => {
                let look = self.look(ContentKind::Scripture, None);
                let presentation =
                    Presentation::from_scripture(&passage, &translations, options, look);
                self.preview = Some(unscheduled(presentation));
            }
            Command::PreviewMedia { media } => {
                let look = self.look(ContentKind::Text, None);
                self.preview = Some(unscheduled(Presentation::from_media(&media, look)));
            }
            Command::SelectPreviewSlide { index } => {
                let pane = self.preview.as_mut().ok_or(EngineError::NothingInPreview)?;
                select(pane, index)?;
            }
            Command::GoLive => {
                let pane = self.preview.clone().ok_or(EngineError::NothingInPreview)?;
                self.go_live(pane);
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
            Command::SetTransition { transition } => self.transition = transition,
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

            // ── Themes ────────────────────────────────────────────────────
            Command::SetThemes { library } => {
                self.themes = *library;
                for pane in [&mut self.preview, &mut self.live].into_iter().flatten() {
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
                self.preview = Some(self.pane_for(item, StartAt::First));
            }
            Command::ShowScheduleItem { id } => {
                let item = self
                    .schedule
                    .get(id)
                    .ok_or(EngineError::UnknownScheduleItem)?;
                let pane = self.pane_for(item, StartAt::First);
                self.go_live(pane);
            }
            Command::RefreshSong { song } => self.refresh_song(&song),
        }
        Ok(())
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
        let presentation = item.presentation(&self.themes);
        for pane in [&mut self.preview, &mut self.live].into_iter().flatten() {
            if pane.schedule_item == Some(id) {
                let last = presentation.slides.len().saturating_sub(1);
                pane.presentation = presentation.clone();
                pane.selected = pane.selected.min(last);
            }
        }
    }

    /// Makes `pane` live. If it came from the schedule, the following item
    /// is loaded into preview so the operator can see what is next.
    fn go_live(&mut self, pane: PaneState) {
        let next = pane
            .schedule_item
            .and_then(|id| self.schedule.after(id))
            .map(|item| self.pane_for(item, StartAt::First));
        if next.is_some() {
            self.preview = next;
        }
        self.live = Some(pane);
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
        let presentation = item.presentation(&self.themes);
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
        for pane in [&mut self.preview, &mut self.live].into_iter().flatten() {
            let shows_song = matches!(
                pane.presentation.source,
                PresentationSource::Song { song_id } if song_id == song.id
            );
            if shows_song && pane.schedule_item.is_none() {
                pane.presentation =
                    Presentation::from_song(song, look.clone(), SlideOptions::default());
                let last = pane.presentation.slides.len().saturating_sub(1);
                pane.selected = pane.selected.min(last);
            }
        }
    }

    /// Recomputes the output state. Returns it (with a new revision) if it
    /// differs from what renderers last received.
    fn refresh_output(&mut self) -> Option<OutputState> {
        let frame = self
            .live
            .as_ref()
            .and_then(|pane| pane.presentation.frame(pane.selected));
        self.follow_live_video(frame.as_ref());

        let unchanged = frame == self.output.frame
            && self.flags == self.output.flags
            && self.transition == self.output.transition
            && self.video == self.output.video
            && self.logo == self.output.logo;
        if unchanged {
            return None;
        }

        self.output = OutputState {
            frame,
            flags: self.flags,
            transition: self.transition,
            video: self.video,
            logo: self.logo.clone(),
            revision: self.output.revision + 1,
        };
        Some(self.output.clone())
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
        }
    }

    fn video_background(media: &MediaItem) -> Background {
        Background::Video {
            media: media.as_ref(),
            fit: MediaFit::Cover,
            looping: true,
            play_audio: false,
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
}
