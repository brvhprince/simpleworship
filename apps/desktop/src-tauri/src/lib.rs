//! # SimpleWorship desktop app
//!
//! A thin shell around `sw-core` and `sw-db`:
//!
//! - [`state`] holds the engine and the library database.
//! - [`commands`] are the functions the web UI calls over Tauri IPC.
//! - [`windows`] opens projector (output) and stage display windows on the
//!   right monitors.
//!
//! The UI never changes state directly. It sends an engine
//! [`Command`](sw_core::Command), and every window learns what changed from
//! the [`EngineEvent`](sw_core::EngineEvent)s broadcast on
//! [`commands::engine::ENGINE_EVENT`].

mod commands;
mod error;
mod state;
mod windows;

use tauri::Manager;

use crate::state::AppState;

/// Builds and runs the application. Returns when the last window closes.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::load(app.handle())?;
            app.manage(state);
            app.manage(windows::OutputPlacements::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::engine::engine_dispatch,
            commands::engine::engine_snapshot,
            commands::songs::songs_search,
            commands::songs::song_get,
            commands::songs::song_save,
            commands::songs::song_delete,
            commands::songs::song_presentation,
            commands::songs::song_show_in_preview,
            commands::text::text_show_in_preview,
            commands::outputs::monitors_list,
            commands::outputs::output_open,
            commands::outputs::output_close,
        ])
        .on_window_event(windows::on_window_event)
        .run(tauri::generate_context!())
        .expect("SimpleWorship failed to start");
}
