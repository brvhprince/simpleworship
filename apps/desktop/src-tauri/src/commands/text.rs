//! Showing free text typed by the operator.

use sw_core::{Command, Presentation, Theme};
use tauri::{AppHandle, State};

use crate::commands::engine::dispatch;
use crate::error::AppResult;
use crate::state::AppState;

/// Turns `text` into slides (split at blank lines) and loads them into the
/// preview pane.
#[tauri::command]
pub fn text_show_in_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    title: String,
    text: String,
) -> AppResult<()> {
    let presentation = Presentation::from_text(title, &text, Theme::default());
    dispatch(
        &app,
        &state,
        Command::ShowInPreview {
            presentation: Box::new(presentation),
        },
    )
}
