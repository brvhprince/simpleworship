//! Importing presentations (PDF, and PowerPoint/Keynote through
//! LibreOffice) and showing them.
//!
//! The webview draws each PDF page (with PDF.js) and sends it back as an
//! image, so no PDF library has to be installed on any platform:
//!
//! 1. [`presentation_prepare`] puts a PDF where the webview can read it,
//!    converting other formats first.
//! 2. [`presentation_create`] starts an empty deck.
//! 3. [`presentation_add_page`] stores each drawn page (raw bytes).
//! 4. [`presentation_finish`] removes the temporary PDF.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sw_core::themes::ContentKind;
use sw_core::{
    Command, Id, ItemContent, ItemLook, MediaItem, MediaKind, Presentation, ScheduleItem, SlideDeck,
};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::{files, office};

/// Broadcast whenever presentations are added or removed.
pub const PRESENTATIONS_EVENT: &str = "presentations://changed";

/// A PDF ready for the webview to read.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct PreparedPdf {
    /// Its file name in the media folder.
    pub file: String,
    /// The presentation's name (the original file name).
    pub name: String,
}

/// Every imported presentation, newest first.
#[tauri::command]
pub fn presentations_list(state: State<'_, AppState>) -> AppResult<Vec<SlideDeck>> {
    Ok(state.database().presentations().list()?)
}

/// Whether PowerPoint and Keynote files can be imported (LibreOffice is
/// installed).
#[tauri::command]
pub fn presentation_can_convert() -> bool {
    office::find_libreoffice().is_some()
}

/// Puts the file, as a PDF, in the media folder for the webview to read.
/// Other formats are converted with LibreOffice first.
#[tauri::command]
pub async fn presentation_prepare(app: AppHandle, path: String) -> AppResult<PreparedPdf> {
    let prepare = move || -> AppResult<PreparedPdf> {
        let state = app.state::<AppState>();
        let source = PathBuf::from(&path);
        let name = source
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Presentation".into());
        let is_pdf = source
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));

        let file = format!("import-{}.pdf", Id::new());
        let destination = state.media_folders.media.join(&file);
        if is_pdf {
            fs::copy(&source, &destination)?;
        } else {
            let scratch = std::env::temp_dir().join(format!("simpleworship-{}", Id::new()));
            fs::create_dir_all(&scratch)?;
            let converted = office::convert_to_pdf(&source, &scratch);
            let result = converted.and_then(|pdf| Ok(fs::copy(pdf, &destination).map(|_| ())?));
            let _ = fs::remove_dir_all(&scratch);
            result?;
        }
        Ok(PreparedPdf { file, name })
    };
    tauri::async_runtime::spawn_blocking(prepare).await?
}

/// Starts an empty presentation and returns its id.
#[tauri::command]
pub fn presentation_create(state: State<'_, AppState>, name: String) -> AppResult<Id> {
    Ok(state.database().presentations().create(name.trim())?)
}

/// Stores one drawn page. The body is the image's raw bytes; the headers
/// `x-deck` and `x-page` say which presentation and which page (from 0).
#[tauri::command]
pub fn presentation_add_page(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> AppResult<MediaItem> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::Conversion("the page image did not arrive".into()));
    };
    let header = |name: &str| {
        request
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| AppError::Conversion(format!("missing {name}")))
    };
    let deck: Id = header("x-deck")?
        .parse()
        .map_err(|_| AppError::PresentationNotFound)?;
    let page: u32 = header("x-page")?.parse().unwrap_or(0);

    let id = Id::new();
    let file = format!("{id}.jpg");
    files::write_atomically(&state.media_folders.media.join(&file), bytes)?;
    let size = imagesize::blob_size(bytes).ok();

    let item = MediaItem {
        id,
        kind: MediaKind::Image,
        // Zero-padded, so pages sort in order.
        name: format!("Page {:03}", page + 1),
        file,
        width: size.and_then(|s| u32::try_from(s.width).ok()),
        height: size.and_then(|s| u32::try_from(s.height).ok()),
        duration_ms: None,
        size_bytes: bytes.len() as u64,
        created_at: sw_core::time::now(),
        collection: Some(deck),
    };
    state.database().media().save(&item)?;
    Ok(item)
}

/// Removes the temporary PDF once every page is stored, or after a failed
/// import (then `deck` is also deleted).
#[tauri::command]
pub fn presentation_finish(
    app: AppHandle,
    state: State<'_, AppState>,
    file: String,
    failed_deck: Option<Id>,
) -> AppResult<()> {
    if file.starts_with("import-") && file.ends_with(".pdf") && !file.contains(['/', '\\']) {
        let _ = fs::remove_file(state.media_folders.media.join(&file));
    }
    if let Some(deck) = failed_deck {
        delete_deck(&state, deck)?;
    }
    announce(&app);
    Ok(())
}

/// Deletes a presentation and its page images.
#[tauri::command]
pub fn presentation_delete(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    delete_deck(&state, id)?;
    announce(&app);
    Ok(())
}

/// Shows a presentation in the preview pane.
#[tauri::command]
pub fn presentation_show_in_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
) -> AppResult<()> {
    let deck = find_deck(&state, id)?;
    let look = state
        .engine()
        .themes()
        .resolve(ContentKind::Text, None, &ItemLook::default());
    let presentation = Presentation::from_pages(&deck.name, &deck.pages, look);
    dispatch(
        &app,
        &state,
        Command::ShowInPreview {
            presentation: Box::new(presentation),
        },
    )
}

/// Adds a presentation to the schedule at `index` (or at the end).
#[tauri::command]
pub fn schedule_add_presentation(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    index: Option<usize>,
) -> AppResult<()> {
    let deck = find_deck(&state, id)?;
    let item = ScheduleItem::new(ItemContent::Slides {
        name: deck.name,
        pages: deck.pages,
    });
    dispatch(
        &app,
        &state,
        Command::AddScheduleItem {
            item: Box::new(item),
            index,
        },
    )
}

fn find_deck(state: &AppState, id: Id) -> AppResult<SlideDeck> {
    state
        .database()
        .presentations()
        .list()?
        .into_iter()
        .find(|deck| deck.id == id)
        .ok_or(AppError::PresentationNotFound)
}

fn delete_deck(state: &AppState, id: Id) -> AppResult<()> {
    for page in state.database().presentations().delete(id)? {
        crate::media::remove_files(&state.media_folders, &page);
    }
    Ok(())
}

fn announce(app: &AppHandle) {
    if let Err(error) = app.emit(PRESENTATIONS_EVENT, ()) {
        eprintln!("could not announce presentation changes: {error}");
    }
}
