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
        if let EngineEvent::ScheduleChanged { schedule } = &event {
            schedule_changed(app, state, schedule);
        }
        app.emit(ENGINE_EVENT, &event)?;
    }
    Ok(())
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
