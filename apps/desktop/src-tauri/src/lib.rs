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
//! - [`import_sources`] finds libraries from other worship software to import.
//! - [`media`] copies images and videos into the media library;
//!   [`schedule_archive`] carries themes and media inside saved schedules.
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
mod import_sources;
mod media;
mod menu;
mod office;
mod recent;
mod schedule_archive;
mod schedule_file;
mod settings;
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

            // Give the engine the theme library and the logo.
            let state = app.state::<AppState>();
            commands::themes::sync_themes(app.handle(), &state)?;
            commands::themes::sync_logo(app.handle(), &state)?;
            commands::themes::sync_settings(app.handle(), &state)?;
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
            commands::themes::theme_save,
            commands::themes::theme_delete,
            commands::themes::theme_set_defaults,
            commands::themes::fonts_list,
            commands::themes::logo_set,
            commands::media::media_folders,
            commands::media::media_list,
            commands::media::media_import,
            commands::media::media_update_details,
            commands::media::media_save_thumbnail,
            commands::media::media_rename,
            commands::media::media_delete,
            commands::media::media_show_in_preview,
            commands::media::schedule_add_media,
            commands::import::import_scan,
            commands::import::import_first_launch,
            commands::import::import_preview,
            commands::import::import_commit,
            commands::schedule::schedule_add_countdown,
            commands::schedule::schedule_add_playlist,
            commands::presentations::presentations_list,
            commands::presentations::presentation_can_convert,
            commands::presentations::presentation_prepare,
            commands::presentations::presentation_create,
            commands::presentations::presentation_add_page,
            commands::presentations::presentation_finish,
            commands::presentations::presentation_delete,
            commands::presentations::presentation_show_in_preview,
            commands::presentations::schedule_add_presentation,
            commands::settings::settings_get,
            commands::settings::settings_save,
            commands::settings::usage_report,
            commands::settings::usage_export_csv,
            commands::text::text_show_in_preview,
            commands::outputs::monitors_list,
            commands::outputs::output_open,
            commands::outputs::output_close,
        ])
        .on_window_event(windows::on_window_event)
        .run(tauri::generate_context!())
        .expect("SimpleWorship failed to start");
}
