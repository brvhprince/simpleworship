//! Driving the presentation engine.

use sw_core::{Command, EngineSnapshot};
use tauri::{AppHandle, Emitter, State};

use crate::error::AppResult;
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

/// Applies `command` and emits its events. Shared by commands that build a
/// presentation first (songs, text) and then hand it to the engine.
pub fn dispatch(app: &AppHandle, state: &AppState, command: Command) -> AppResult<()> {
    // Hold the lock while emitting so that events from two commands sent at
    // the same moment can never reach the windows out of order.
    let mut engine = state.engine();
    for event in engine.dispatch(command)? {
        app.emit(ENGINE_EVENT, &event)?;
    }
    Ok(())
}
