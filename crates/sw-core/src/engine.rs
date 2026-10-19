//! The presentation engine: preview, live and output state.
//!
//! The engine mirrors the two panes of the operator window:
//!
//! - **Preview**: what the operator is about to show. Changing it never
//!   affects the screens.
//! - **Live**: what the congregation sees right now.
//!
//! On top of the live slide sit the [`DisplayFlags`] (black, logo, hide text,
//! hide background). From the live slide and the flags the engine derives one
//! [`OutputState`], which is all a renderer needs.
//!
//! The engine is a plain struct with no I/O and no threads. The host (the
//! desktop app) owns it behind a mutex, feeds it [`Command`]s and broadcasts
//! the resulting [`EngineEvent`]s to every window.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::command::{Command, EngineEvent};
use crate::output::{DisplayFlags, OutputState, Transition};
use crate::presentation::Presentation;

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
}

/// A full copy of the engine state, sent to a window when it opens so it can
/// draw immediately without waiting for the next event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct EngineSnapshot {
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
}

/// The presentation engine. See the [module docs](self).
#[derive(Debug, Default)]
pub struct Engine {
    preview: Option<PaneState>,
    live: Option<PaneState>,
    flags: DisplayFlags,
    transition: Transition,
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

impl Engine {
    /// Creates an engine with nothing in preview or live.
    pub fn new() -> Self {
        Self::default()
    }

    /// A full copy of the current state.
    pub fn snapshot(&self) -> EngineSnapshot {
        EngineSnapshot {
            preview: self.preview.clone(),
            live: self.live.clone(),
            output: self.output.clone(),
        }
    }

    /// Applies a command and returns the events describing what changed.
    ///
    /// Commands that change nothing (pressing "next" on the last slide,
    /// turning black on when it is already on) return no events.
    pub fn dispatch(&mut self, command: Command) -> Result<Vec<EngineEvent>, EngineError> {
        let before_preview = self.preview.clone();
        let before_live = self.live.clone();

        self.apply(command)?;

        let mut events = Vec::new();
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
            Command::ShowInPreview { presentation } => {
                self.preview = Some(PaneState {
                    presentation: *presentation,
                    selected: 0,
                });
            }
            Command::SelectPreviewSlide { index } => {
                let pane = self.preview.as_mut().ok_or(EngineError::NothingInPreview)?;
                select(pane, index)?;
            }
            Command::GoLive => {
                let pane = self.preview.clone().ok_or(EngineError::NothingInPreview)?;
                self.live = Some(pane);
            }
            Command::SelectLiveSlide { index } => {
                select(self.live_mut()?, index)?;
            }
            Command::NextSlide => {
                let pane = self.live_mut()?;
                if pane.selected + 1 < pane.presentation.slides.len() {
                    pane.selected += 1;
                }
            }
            Command::PreviousSlide => {
                let pane = self.live_mut()?;
                pane.selected = pane.selected.saturating_sub(1);
            }
            Command::JumpToSection { shortcut } => {
                let pane = self.live_mut()?;
                if let Some(index) = pane.presentation.find_section(&shortcut) {
                    pane.selected = index;
                }
            }
            Command::ClearLive => self.live = None,
            Command::SetBlack { on } => self.flags.black = on,
            Command::SetLogo { on } => self.flags.logo = on,
            Command::SetHideText { on } => self.flags.hide_text = on,
            Command::SetHideBackground { on } => self.flags.hide_background = on,
            Command::SetTransition { transition } => self.transition = transition,
        }
        Ok(())
    }

    fn live_mut(&mut self) -> Result<&mut PaneState, EngineError> {
        self.live.as_mut().ok_or(EngineError::NothingLive)
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn next_and_previous_stop_at_the_ends() {
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
    }
}
