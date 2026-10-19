//! Monitors and output windows.
//!
//! These commands are `async` because creating a window from a synchronous
//! command can deadlock on Windows (see the Tauri window docs).

use tauri::AppHandle;

use crate::error::AppResult;
use crate::windows::{self, MonitorInfo, OutputKind, OutputTarget};

/// Lists the connected monitors.
#[tauri::command]
pub async fn monitors_list(app: AppHandle) -> AppResult<Vec<MonitorInfo>> {
    windows::list_monitors(&app)
}

/// Opens (or moves) the projector or stage window.
#[tauri::command]
pub async fn output_open(app: AppHandle, kind: OutputKind, target: OutputTarget) -> AppResult<()> {
    windows::open_output(&app, kind, target)
}

/// Closes the projector or stage window.
#[tauri::command]
pub async fn output_close(app: AppHandle, kind: OutputKind) -> AppResult<()> {
    windows::close_output(&app, kind)
}
