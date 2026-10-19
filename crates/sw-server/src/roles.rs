//! What each kind of connected device may do.
//!
//! A device's role is chosen by the operator when it connects, and every
//! command it sends is checked here before the engine sees it, so a phone
//! can never do more than its role allows, whatever it sends.

use serde::{Deserialize, Serialize};
use sw_core::{Command, EngineSnapshot, ItemContent, PresentationSource};
use ts_rs::TS;

/// What a connected device is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "remote.ts")]
pub enum Role {
    /// A second operator: everything the operator window can do from a
    /// phone (live controls, schedule, songs, scripture, alerts, stage).
    Full,
    /// The preacher: move through the live slides, jump to a section, and
    /// black the screen.
    Presenter,
    /// The worship leader: only songs. Move through the live song, jump to
    /// a verse or chorus, and send the songs in the schedule live.
    WorshipLeader,
    /// Nursery and ushers: show and clear alerts, nothing else.
    Alerts,
    /// A tablet on the pulpit or for the band: the stage display, view only.
    Stage,
}

impl Role {
    /// Every role, in the order the operator is offered them.
    pub const ALL: [Role; 5] = [
        Role::Full,
        Role::Presenter,
        Role::WorshipLeader,
        Role::Alerts,
        Role::Stage,
    ];
}

/// Whether a device with `role` may send `command`, given the engine's
/// current state (the worship leader may only work with songs).
pub fn may_dispatch(role: Role, command: &Command, state: &EngineSnapshot) -> bool {
    match role {
        Role::Full => true,
        Role::Presenter => matches!(
            command,
            Command::NextSlide
                | Command::PreviousSlide
                | Command::SelectLiveSlide { .. }
                | Command::JumpToSection { .. }
                | Command::SetBlack { .. }
        ),
        Role::WorshipLeader => match command {
            // Moving through what is live, if it is a song.
            Command::NextSlide
            | Command::PreviousSlide
            | Command::SelectLiveSlide { .. }
            | Command::JumpToSection { .. } => live_is_song(state),
            // Sending a song from the schedule live.
            Command::PreviewScheduleItem { id } | Command::ShowScheduleItem { id } => state
                .schedule
                .get(*id)
                .is_some_and(|item| matches!(item.content, ItemContent::Song { .. })),
            _ => false,
        },
        Role::Alerts => matches!(command, Command::ShowAlert { .. } | Command::ClearAlert),
        Role::Stage => false,
    }
}

fn live_is_song(state: &EngineSnapshot) -> bool {
    state
        .live
        .as_ref()
        .is_some_and(|pane| matches!(pane.presentation.source, PresentationSource::Song { .. }))
}

/// Questions a device may ask the host (see `Host::call`), by role.
pub fn may_call(role: Role, method: &str) -> bool {
    match role {
        Role::Full => true,
        Role::Presenter | Role::WorshipLeader => method == "stage.options",
        Role::Alerts => method == "alerts.templates",
        Role::Stage => method == "stage.options",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sw_core::{Engine, Id, ScheduleItem, Song};

    fn state_with(items: Vec<ScheduleItem>) -> EngineSnapshot {
        let mut engine = Engine::new();
        engine
            .dispatch(Command::ReplaceSchedule {
                schedule: Box::new(sw_core::Schedule { items }),
            })
            .unwrap();
        engine.snapshot()
    }

    #[test]
    fn the_preacher_can_move_slides_and_black_out_but_not_send_things_live() {
        let state = state_with(Vec::new());
        assert!(may_dispatch(Role::Presenter, &Command::NextSlide, &state));
        assert!(may_dispatch(
            Role::Presenter,
            &Command::SetBlack { on: true },
            &state
        ));
        assert!(!may_dispatch(Role::Presenter, &Command::GoLive, &state));
        assert!(!may_dispatch(
            Role::Presenter,
            &Command::SetLogo { on: true },
            &state
        ));
    }

    #[test]
    fn the_worship_leader_only_handles_songs() {
        let song = ScheduleItem::new(ItemContent::Song {
            song: Song::new("Amazing Grace"),
        });
        let text = ScheduleItem::new(ItemContent::Text {
            title: "Notices".into(),
            text: "Welcome".into(),
        });
        let (song_id, text_id) = (song.id, text.id);
        let mut engine = Engine::new();
        engine
            .dispatch(Command::ReplaceSchedule {
                schedule: Box::new(sw_core::Schedule {
                    items: vec![song, text],
                }),
            })
            .unwrap();

        let state = engine.snapshot();
        let show = |id: Id| Command::ShowScheduleItem { id };
        assert!(may_dispatch(Role::WorshipLeader, &show(song_id), &state));
        assert!(!may_dispatch(Role::WorshipLeader, &show(text_id), &state));

        // While the notices are live, Next belongs to the operator.
        engine.dispatch(show(text_id)).unwrap();
        let state = engine.snapshot();
        assert!(!may_dispatch(
            Role::WorshipLeader,
            &Command::NextSlide,
            &state
        ));

        engine.dispatch(show(song_id)).unwrap();
        let state = engine.snapshot();
        assert!(may_dispatch(
            Role::WorshipLeader,
            &Command::NextSlide,
            &state
        ));
    }

    #[test]
    fn alerts_and_stage_devices_are_limited() {
        let state = state_with(Vec::new());
        assert!(may_dispatch(Role::Alerts, &Command::ClearAlert, &state));
        assert!(!may_dispatch(Role::Alerts, &Command::NextSlide, &state));
        assert!(!may_dispatch(Role::Stage, &Command::NextSlide, &state));
        assert!(may_call(Role::Alerts, "alerts.templates"));
        assert!(!may_call(Role::Alerts, "songs.search"));
        assert!(may_call(Role::Full, "songs.search"));
    }
}
