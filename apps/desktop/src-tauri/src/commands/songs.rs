//! The song library.

use sw_core::{Command, Id, Presentation, SlideOptions, Song, SongSummary, Theme};
use tauri::{AppHandle, State};

use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Searches titles, authors and lyrics. An empty query lists every song.
#[tauri::command]
pub fn songs_search(state: State<'_, AppState>, query: String) -> AppResult<Vec<SongSummary>> {
    Ok(state.database().songs().search(&query)?)
}

/// Loads one song for editing.
#[tauri::command]
pub fn song_get(state: State<'_, AppState>, id: Id) -> AppResult<Song> {
    state
        .database()
        .songs()
        .get(id)?
        .ok_or(AppError::SongNotFound)
}

/// Creates or updates a song and returns the saved copy.
#[tauri::command]
pub fn song_save(state: State<'_, AppState>, song: Song) -> AppResult<Song> {
    Ok(state.database().songs().save(&song)?)
}

/// Deletes a song. Returns `false` if it was already gone.
#[tauri::command]
pub fn song_delete(state: State<'_, AppState>, id: Id) -> AppResult<bool> {
    Ok(state.database().songs().delete(id)?)
}

/// Builds the slides for a (possibly unsaved) song, so the editor can show
/// how the lyrics will be split while the operator types.
#[tauri::command]
pub fn song_presentation(song: Song) -> Presentation {
    presentation_for(&song)
}

/// Loads a song from the library into the preview pane.
#[tauri::command]
pub fn song_show_in_preview(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    let song = state
        .database()
        .songs()
        .get(id)?
        .ok_or(AppError::SongNotFound)?;
    let presentation = presentation_for(&song);
    dispatch(
        &app,
        &state,
        Command::ShowInPreview {
            presentation: Box::new(presentation),
        },
    )
}

/// The one place that decides how songs look. Per-song themes and slide
/// options arrive with the theme designer (M5).
fn presentation_for(song: &Song) -> Presentation {
    Presentation::from_song(song, Theme::default(), SlideOptions::default())
}
