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
//! The engine is a plain struct with no I/O and no threads. The host (the
//! desktop app) owns it behind a mutex, feeds it [`Command`]s and broadcasts
//! the resulting [`EngineEvent`]s to every window.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::command::{Command, EngineEvent};
use crate::id::Id;
use crate::output::{DisplayFlags, OutputState, Transition};
use crate::presentation::{Presentation, PresentationSource};
use crate::schedule::{Schedule, ScheduleItem};
use crate::slides::SlideOptions;
use crate::song::Song;
use crate::theme::Theme;

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
#[derive(Debug, Default)]
pub struct Engine {
    schedule: Schedule,
    preview: Option<PaneState>,
    live: Option<PaneState>,
    flags: DisplayFlags,
    transition: Transition,
    /// The theme new presentations are drawn with. Theme selection arrives
    /// with the theme designer; until then this is always the default.
    theme: Theme,
    /// Set by any command that changes the schedule, so [`Engine::dispatch`]
    /// knows to announce it.
    schedule_changed: bool,
    /// The output state last announced to renderers.
    output: OutputState,
}

impl Default for OutputState {
    /// A black screen with nothing live.
    fn default() -> Self {
        Self {
            frame: None,
            flags: DisplayFlags::default(),
            transition: Transition::default(),
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

    /// A full copy of the current state.
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            schedule: self.schedule.clone(),
            preview: self.preview.clone(),
            live: self.live.clone(),
            output: self.output.clone(),
        }
    }

    /// The current schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Applies a command and returns the events describing what changed.
    ///
    /// Commands that change nothing (pressing "next" on the last slide,
    /// turning black on when it is already on) return no events.
    pub fn dispatch(&mut self, command: Command) -> Result<Vec<EngineEvent>, EngineError> {
        let before_preview = self.preview.clone();
        let before_live = self.live.clone();
        self.schedule_changed = false;

        self.apply(command)?;

        let mut events = Vec::new();
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
                self.preview = Some(PaneState {
                    presentation: *presentation,
                    selected: 0,
                    schedule_item: None,
                });
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
        let presentation = item.presentation(&self.theme);
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

    /// Applies a library edit to the schedule and to any pane showing the song.
    fn refresh_song(&mut self, song: &Song) {
        if !self.schedule.refresh_song(song).is_empty() {
            self.schedule_changed = true;
        }
        for pane in [&mut self.preview, &mut self.live].into_iter().flatten() {
            refresh_pane(pane, song);
        }
    }

    /// Recomputes the output state. Returns it (with a new revision) if it
    /// differs from what renderers last received.
    fn refresh_output(&mut self) -> Option<OutputState> {
        let frame = self
            .live
            .as_ref()
            .and_then(|pane| pane.presentation.frame(pane.selected));

        let unchanged = frame == self.output.frame
            && self.flags == self.output.flags
            && self.transition == self.output.transition;
        if unchanged {
            return None;
        }

        self.output = OutputState {
            frame,
            flags: self.flags,
            transition: self.transition,
            revision: self.output.revision + 1,
        };
        Some(self.output.clone())
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

/// Rebuilds a pane's slides if it shows `song`, keeping the same slide
/// selected (or the last one, if the song got shorter).
fn refresh_pane(pane: &mut PaneState, song: &Song) {
    let shows_song = matches!(
        pane.presentation.source,
        PresentationSource::Song { song_id } if song_id == song.id
    );
    if !shows_song {
        return;
    }

    let theme = pane.presentation.theme.clone();
    pane.presentation = Presentation::from_song(song, theme, SlideOptions::default());
    let last = pane.presentation.slides.len().saturating_sub(1);
    pane.selected = pane.selected.min(last);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::ItemContent;

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
}
