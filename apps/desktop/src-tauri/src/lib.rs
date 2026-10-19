//! # SimpleWorship desktop app
//!
//! A thin shell around `sw-core` and `sw-db`:
//!
//! - [`state`] holds the engine and the library database.
//! - [`commands`] are the functions the web UI calls over Tauri IPC.
//! - [`windows`] opens projector (output) and stage display windows on the
//!   right monitors.
//! - [`schedule_file`], [`autosave`] and [`recent`] handle `.sws` files:
//!   unsaved changes, crash recovery and "Open Recent".
//! - [`menu`] builds the native menu bar.
//! - [`bibles`] imports Bibles in the background, including the bundled ones.
//!
//! The UI never changes state directly. It sends an engine
//! [`Command`](sw_core::Command), and every window learns what changed from
//! the [`EngineEvent`](sw_core::EngineEvent)s broadcast on
//! [`commands::engine::ENGINE_EVENT`].

mod autosave;
mod bibles;
mod commands;
mod error;
mod files;
mod menu;
mod recent;
mod schedule_file;
mod state;
mod windows;

use tauri::Manager;

use crate::state::AppState;

/// Builds and runs the application. Returns when the last window closes.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::load(app.handle())?;
            let recent = state.recent().list();
            app.manage(state);
            app.manage(windows::OutputPlacements::default());

            menu::install(app.handle())?;
            menu::update_recent(app.handle(), &recent)?;
            bibles::install_bundled(app.handle());
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
            commands::schedule::schedule_add_song,
            commands::schedule::schedule_add_text,
            commands::schedule::schedule_missing_songs,
            commands::schedule::schedule_add_missing_songs,
            commands::schedule::schedule_file_status,
            commands::schedule::schedule_new,
            commands::schedule::schedule_open,
            commands::schedule::schedule_save,
            commands::schedule::schedule_recent,
            commands::schedule::autosave_leftover,
            commands::schedule::autosave_restore,
            commands::schedule::autosave_discard,
            commands::scripture::bibles_list,
            commands::scripture::bible_import,
            commands::scripture::bible_rename,
            commands::scripture::bible_delete,
            commands::scripture::bible_outline,
            commands::scripture::bible_chapter,
            commands::scripture::bible_search,
            commands::scripture::scripture_parse,
            commands::scripture::scripture_show_in_preview,
            commands::scripture::schedule_add_scripture,
            commands::text::text_show_in_preview,
            commands::outputs::monitors_list,
            commands::outputs::output_open,
            commands::outputs::output_close,
        ])
        .on_window_event(windows::on_window_event)
        .run(tauri::generate_context!())
        .expect("SimpleWorship failed to start");
}
