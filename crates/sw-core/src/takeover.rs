//! Carrying on a service from another computer (the joined computer's
//! "Take Over", when the main computer stops answering).
//!
//! A computer that joined another keeps a copy of its service, kept up to
//! date by the same events its windows receive ([`EngineSnapshot::apply`]).
//! Each event carries a whole part of the state (the schedule, what is
//! live, the screens' state…), so applying one is just replacing that part.
//!
//! The other computer's times (where a video was, when a countdown ends)
//! are on its clock; [`EngineSnapshot::shifted`] moves them onto this one
//! before [`Engine::take_over`](crate::Engine::take_over) carries on.

use crate::command::EngineEvent;
use crate::engine::EngineSnapshot;
use crate::time::Timestamp;

impl EngineSnapshot {
    /// Brings the copy up to date with an event from the other computer.
    /// Preview changes count only if they are this computer's own (the
    /// joined computer passes those on as the local operator's).
    pub fn apply(&mut self, event: &EngineEvent) {
        match event {
            EngineEvent::ThemesChanged { themes } => self.themes = themes.clone(),
            EngineEvent::ScheduleChanged { schedule } => self.schedule = schedule.clone(),
            EngineEvent::PreviewChanged { operator, preview } => {
                if *operator == crate::engine::LOCAL_OPERATOR {
                    self.preview = preview.clone();
                }
            }
            EngineEvent::LiveChanged { live } => self.live = live.clone(),
            EngineEvent::OutputChanged { output } => self.output = (**output).clone(),
            EngineEvent::StageChanged { stage } => self.stage = stage.clone(),
            EngineEvent::AudioChanged { audio } => self.audio = audio.clone(),
            EngineEvent::TransitionsChanged { transitions } => self.transitions = *transitions,
            EngineEvent::StreamSettingsChanged { settings } => self.stream = *settings,
            // Reports, not state.
            EngineEvent::SongWentLive { .. } => {}
        }
    }

    /// The same state with every time moved by `-offset_ms`: from a clock
    /// `offset_ms` ahead of this computer's onto this computer's.
    pub fn shifted(mut self, offset_ms: i64) -> Self {
        let shift = |time: &mut Timestamp| *time -= offset_ms;
        if let Some(video) = &mut self.output.video {
            shift(&mut video.anchor);
        }
        if let Some(alert) = &mut self.output.alert {
            shift(&mut alert.started_at);
        }
        if let Some(countdown) = self
            .output
            .frame
            .as_mut()
            .and_then(|frame| frame.countdown.as_mut())
        {
            shift(&mut countdown.ends_at);
        }
        if let Some(countdown) = self
            .output
            .stream
            .frame
            .as_mut()
            .and_then(|frame| frame.countdown.as_mut())
        {
            shift(&mut countdown.ends_at);
        }
        if let Some(timer) = &mut self.stage.timer {
            shift(&mut timer.anchor);
        }
        shift(&mut self.audio.anchor);
        if let Some(fade) = &mut self.audio.fade {
            shift(&mut fade.started_at);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::command::{Command, EngineEvent};
    use crate::engine::{Engine, LOCAL_OPERATOR};
    use crate::id::Id;

    /// An engine whose clock always reads the same time.
    fn engine() -> Engine {
        Engine::with_clock(|| 10_000)
    }

    #[test]
    fn a_copy_follows_the_events() {
        let mut main = engine();
        let mut copy = main.snapshot();
        for command in [
            Command::PreviewText {
                title: "Welcome".into(),
                text: "Welcome".into(),
            },
            Command::GoLive,
            Command::SetBlack { on: true },
        ] {
            for event in main.dispatch(command).unwrap() {
                copy.apply(&event);
            }
        }
        let live = main.snapshot();
        assert_eq!(copy.live, live.live);
        assert_eq!(copy.output, live.output);
    }

    #[test]
    fn other_operators_previews_are_not_copied() {
        let mut copy = engine().snapshot();
        copy.apply(&EngineEvent::PreviewChanged {
            operator: Id::new(),
            preview: engine().snapshot().live,
        });
        assert_eq!(copy.preview, None);
        let _ = LOCAL_OPERATOR;
    }

    #[test]
    fn times_move_onto_this_computers_clock() {
        let mut main = engine();
        main.dispatch(Command::StartStageTimer {
            mode: crate::stage::TimerMode::CountDown,
            duration_ms: 60_000,
        })
        .unwrap();
        let service = main.snapshot();
        let anchor = service.stage.timer.as_ref().unwrap().anchor;
        // The other computer's clock was 3 s ahead of this one.
        let shifted = service.shifted(3_000);
        assert_eq!(shifted.stage.timer.unwrap().anchor, anchor - 3_000);
    }

    #[test]
    fn taking_over_carries_on_from_the_same_slide() {
        let mut main = engine();
        main.dispatch(Command::PreviewText {
            title: "Notices".into(),
            text: "One\n\nTwo\n\nThree".into(),
        })
        .unwrap();
        main.dispatch(Command::GoLive).unwrap();
        main.dispatch(Command::NextSlide).unwrap();
        main.dispatch(Command::SetBlack { on: true }).unwrap();

        let mut backup = engine();
        let events = backup.take_over(main.snapshot().shifted(0));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::LiveChanged { .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, EngineEvent::OutputChanged { .. }))
        );

        let carried = backup.snapshot();
        assert_eq!(carried.live.as_ref().unwrap().selected, 1);
        assert!(carried.output.flags.black);
        assert_eq!(carried.output.frame, main.snapshot().output.frame);
        // And it goes on from there.
        backup.dispatch(Command::NextSlide).unwrap();
        assert_eq!(backup.snapshot().live.unwrap().selected, 2);
    }
}
