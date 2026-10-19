//! Showing free text typed by the operator.

use sw_core::Command;
use tauri::{AppHandle, State};

use crate::commands::engine::dispatch;
use crate::error::AppResult;
use crate::state::AppState;

/// Turns `text` into slides (split at blank lines) and loads them into the
/// preview pane, in the default text theme.
#[tauri::command]
pub fn text_show_in_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    title: String,
    text: String,
) -> AppResult<()> {
    dispatch(&app, &state, Command::PreviewText { title, text })
}
