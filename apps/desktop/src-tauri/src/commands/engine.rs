//! Driving the presentation engine.

use std::cell::Cell;

use sw_core::{Command, EngineEvent, EngineSnapshot, Id, LOCAL_OPERATOR, Schedule};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppResult;
use crate::schedule_file;
use crate::state::AppState;

/// The event channel every engine event is broadcast on, to all windows.
/// Keep in sync with `ENGINE_EVENT` in `packages/protocol`.
pub const ENGINE_EVENT: &str = "engine://event";

thread_local! {
    /// Whose command this thread is carrying out: this computer's own
    /// window, or a phone or joined computer (see [`as_operator`]).
    static OPERATOR: Cell<Id> = const { Cell::new(LOCAL_OPERATOR) };
}

/// Runs `work` on behalf of `operator` (a phone or a joined computer):
/// whatever it puts in a preview goes to that operator's own preview pane.
pub fn as_operator<T>(operator: Id, work: impl FnOnce() -> T) -> T {
    let before = OPERATOR.with(|current| current.replace(operator));
    let result = work();
    OPERATOR.with(|current| current.set(before));
    result
}

/// Sends a command to the engine and broadcasts the resulting events. On a
/// computer that joined another, it goes to that computer's engine (see
/// `link.rs`). Async, so waiting for the other computer never holds up the
/// window.
#[tauri::command]
pub async fn engine_dispatch(app: AppHandle, command: Command) -> AppResult<()> {
    if crate::link::is_joined(&app) {
        return crate::link::dispatch(&app, command).await;
    }
    dispatch(&app, &app.state::<AppState>(), command)
}

/// Returns the full engine state, for windows that have just opened: the
/// joined computer's, on a computer that joined another.
#[tauri::command]
pub async fn engine_snapshot(app: AppHandle) -> AppResult<EngineSnapshot> {
    if crate::link::is_joined(&app) {
        return crate::link::snapshot(&app).await;
    }
    Ok(app.state::<AppState>().engine().snapshot())
}

/// Applies `command`, emits its events, and keeps the schedule file status
/// and the autosave up to date. Every command reaches the engine through
/// here, whether it came from the UI or from another host command.
pub fn dispatch(app: &AppHandle, state: &AppState, command: Command) -> AppResult<()> {
    // Hold the lock while emitting so that events from two commands sent at
    // the same moment can never reach the windows out of order.
    let mut engine = state.engine();
    let operator = OPERATOR.with(Cell::get);
    let events = engine.dispatch_as(operator, command)?;
    // A computer that joined another shows that one's service: its own
    // engine still follows its library, but is not shown.
    if crate::link::is_joined(app) {
        return Ok(());
    }
    for event in events {
        match &event {
            EngineEvent::ScheduleChanged { schedule } => schedule_changed(app, state, schedule),
            EngineEvent::SongWentLive { song_id, title } => record_usage(state, *song_id, title),
            _ => {}
        }
        app.emit(ENGINE_EVENT, &event)?;
        crate::remote::publish(app, &event);
    }
    Ok(())
}

/// Logs a song for the CCLI report, on this computer's local date. A
/// failure is only reported: it must never stop a song going live.
fn record_usage(state: &AppState, song_id: sw_core::Id, title: &str) {
    let day = chrono::Local::now().format("%Y-%m-%d").to_string();
    let database = state.database();
    let song = database.songs().get(song_id).ok().flatten();
    if let Err(error) = database.usage().record(song_id, title, song.as_ref(), &day) {
        log::warn!("could not record song usage: {error}");
    }
}

/// Marks the schedule as having unsaved changes and autosaves it.
fn schedule_changed(app: &AppHandle, state: &AppState, schedule: &Schedule) {
    let mut file = state.schedule_file();
    state.autosave.save(schedule.clone(), file.path.clone());

    if !file.dirty {
        file.dirty = true;
        schedule_file::announce(app, &file.status());
    }
}
