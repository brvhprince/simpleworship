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
//! 4. [`presentation_finish`] removes the temporary PDF and saves the deck.
//!
//! Presentations made or changed in the editor are saved whole with
//! [`presentation_save`].

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sw_core::themes::ContentKind;
use sw_core::{
    Command, DeckSlide, Id, ItemContent, ItemLook, MediaItem, MediaKind, Presentation,
    ScheduleItem, SlideDeck,
};
use tauri::ipc::{InvokeBody, Request};
use tauri::{AppHandle, Manager, State};
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

/// Every presentation, most recently changed first.
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
        scaling: sw_core::MediaScaling::Automatic,
    };
    state.database().media().save(&item)?;
    Ok(item)
}

/// Removes the temporary PDF once every page is stored, and saves the
/// new deck (one picture slide per page). After a failed import,
/// `failed_deck` is deleted instead.
#[tauri::command]
pub fn presentation_finish(
    app: AppHandle,
    state: State<'_, AppState>,
    file: String,
    deck: Option<Id>,
    failed: Option<bool>,
) -> AppResult<()> {
    if file.starts_with("import-") && file.ends_with(".pdf") && !file.contains(['/', '\\']) {
        let _ = fs::remove_file(state.media_folders.media.join(&file));
    }
    if let Some(id) = deck {
        if failed.unwrap_or(false) {
            delete_deck(&state, id)?;
        } else {
            let database = state.database();
            if let Some(imported) = database.presentations().get(id)? {
                database.presentations().save(&imported)?;
            }
        }
    }
    announce(&app);
    Ok(())
}

/// One presentation.
#[tauri::command]
pub fn presentation_get(state: State<'_, AppState>, id: Id) -> AppResult<SlideDeck> {
    find_deck(&state, id)
}

/// Saves a presentation from the editor, and updates its copies in the
/// schedule and on screen. Returns it as saved.
#[tauri::command]
pub fn presentation_save(
    app: AppHandle,
    state: State<'_, AppState>,
    deck: SlideDeck,
) -> AppResult<SlideDeck> {
    let saved = state.database().presentations().save(&deck)?;
    dispatch(
        &app,
        &state,
        Command::RefreshDeck {
            deck: Box::new(saved.clone()),
        },
    )?;
    announce(&app);
    Ok(saved)
}

/// Deletes a presentation and its imported page images.
#[tauri::command]
pub fn presentation_delete(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    delete_deck(&state, id)?;
    announce(&app);
    Ok(())
}

/// The slides a deck makes, styled as it would be shown: for the editor's
/// live preview and the Presentations tab's thumbnails. The deck need not
/// be saved.
#[tauri::command]
pub fn deck_presentation(state: State<'_, AppState>, deck: SlideDeck) -> Presentation {
    let engine = state.engine();
    let look = engine.themes().resolve(
        ContentKind::Presentation,
        deck.theme_id,
        &ItemLook::default(),
    );
    Presentation::from_deck(&deck, look, engine.themes())
}

/// A presentation ready for the canvas editor: slides made before it had
/// one get their text turned into a text box, styled by the theme they
/// are shown with.
#[tauri::command]
pub fn deck_for_editing(state: State<'_, AppState>, mut deck: SlideDeck) -> SlideDeck {
    let engine = state.engine();
    let themes = engine.themes();
    let deck_look = themes.resolve(
        ContentKind::Presentation,
        deck.theme_id,
        &ItemLook::default(),
    );
    for slide in &mut deck.slides {
        let theme = slide
            .theme_id
            .and_then(|id| themes.get(id))
            .map_or(&deck_look.theme, |record| &record.theme);
        slide.upgrade(theme);
    }
    deck
}

/// Picture slides for media items, in the order given (for the editor's
/// "Add Pictures"). Sound files are left out.
#[tauri::command]
pub fn deck_picture_slides(
    state: State<'_, AppState>,
    media_ids: Vec<Id>,
) -> AppResult<Vec<DeckSlide>> {
    Ok(media_items(&state, &media_ids)?
        .iter()
        .filter_map(DeckSlide::picture)
        .collect())
}

/// Shows a presentation in the preview pane.
#[tauri::command]
pub fn presentation_show_in_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
) -> AppResult<()> {
    let deck = find_deck(&state, id)?;
    let presentation = deck_presentation(state.clone(), deck);
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
    add_deck_item(&app, &state, deck, index)
}

/// Adds pictures and videos to the schedule as one slideshow item: step
/// through them with the arrow keys, like a song.
#[tauri::command]
pub fn schedule_add_slideshow(
    app: AppHandle,
    state: State<'_, AppState>,
    media_ids: Vec<Id>,
    index: Option<usize>,
) -> AppResult<()> {
    let media = media_items(&state, &media_ids)?;
    let deck = SlideDeck::slideshow(SlideDeck::slideshow_name(&media), &media);
    add_deck_item(&app, &state, deck, index)
}

/// Adds pictures and videos to the end of schedule item `id` (dropped onto
/// it). The item becomes a slideshow if it was a single picture.
#[tauri::command]
pub fn schedule_append_media(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    media_ids: Vec<Id>,
) -> AppResult<()> {
    let slides = media_items(&state, &media_ids)?
        .iter()
        .filter_map(DeckSlide::picture)
        .collect();
    dispatch(&app, &state, Command::AppendSlides { id, slides })
}

/// Adds a presentation's slides to the end of schedule item `id`.
#[tauri::command]
pub fn schedule_append_presentation(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    deck_id: Id,
) -> AppResult<()> {
    let slides = find_deck(&state, deck_id)?
        .slides
        .into_iter()
        // Fresh ids: the same deck may be added to an item twice.
        .map(|slide| DeckSlide {
            id: Id::new(),
            ..slide
        })
        .collect();
    dispatch(&app, &state, Command::AppendSlides { id, slides })
}

fn add_deck_item(
    app: &AppHandle,
    state: &AppState,
    deck: SlideDeck,
    index: Option<usize>,
) -> AppResult<()> {
    let item = ScheduleItem::new(ItemContent::Deck { deck });
    dispatch(
        app,
        state,
        Command::AddScheduleItem {
            item: Box::new(item),
            index,
        },
    )
}

/// The media items with `ids`, in that order.
fn media_items(state: &AppState, ids: &[Id]) -> AppResult<Vec<MediaItem>> {
    let database = state.database();
    ids.iter()
        .map(|&id| database.media().get(id)?.ok_or(AppError::MediaNotFound))
        .collect()
}

fn find_deck(state: &AppState, id: Id) -> AppResult<SlideDeck> {
    state
        .database()
        .presentations()
        .get(id)?
        .ok_or(AppError::PresentationNotFound)
}

fn delete_deck(state: &AppState, id: Id) -> AppResult<()> {
    for page in state.database().presentations().delete(id)? {
        crate::media::remove_files(&state.media_folders, &page);
    }
    Ok(())
}

fn announce(app: &AppHandle) {
    crate::remote::share(app, PRESENTATIONS_EVENT, ());
}
