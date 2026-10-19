//! Driving the presentation engine.

use sw_core::{Command, EngineEvent, EngineSnapshot, Schedule};
use tauri::{AppHandle, Emitter, State};

use crate::error::AppResult;
use crate::schedule_file;
use crate::state::AppState;

/// The event channel every engine event is broadcast on, to all windows.
/// Keep in sync with `ENGINE_EVENT` in `packages/protocol`.
pub const ENGINE_EVENT: &str = "engine://event";

/// Sends a command to the engine and broadcasts the resulting events.
#[tauri::command]
pub fn engine_dispatch(
    app: AppHandle,
    state: State<'_, AppState>,
    command: Command,
) -> AppResult<()> {
    dispatch(&app, &state, command)
}

/// Returns the full engine state, for windows that have just opened.
#[tauri::command]
pub fn engine_snapshot(state: State<'_, AppState>) -> EngineSnapshot {
    state.engine().snapshot()
}

/// Applies `command`, emits its events, and keeps the schedule file status
/// and the autosave up to date. Every command reaches the engine through
/// here, whether it came from the UI or from another host command.
pub fn dispatch(app: &AppHandle, state: &AppState, command: Command) -> AppResult<()> {
    // Hold the lock while emitting so that events from two commands sent at
    // the same moment can never reach the windows out of order.
    let mut engine = state.engine();
    for event in engine.dispatch(command)? {
        match &event {
            EngineEvent::ScheduleChanged { schedule } => schedule_changed(app, state, schedule),
            EngineEvent::SongWentLive { song_id, title } => record_usage(state, *song_id, title),
            _ => {}
        }
        app.emit(ENGINE_EVENT, &event)?;
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
