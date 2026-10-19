//! Scripture: Bibles, references, verse lookup and showing passages.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sw_bible::reference::{self, Passage};
use sw_bible::{BookId, VersePoint, VerseRange};
use sw_core::{
    BibleInfo, BookOutline, Command, Id, ItemContent, ScheduleItem, ScriptureOptions,
    TranslationText, Verse,
};
use tauri::{AppHandle, State};
use ts_rs::TS;

use crate::bibles;
use crate::commands::engine::dispatch;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

// ── Bibles ───────────────────────────────────────────────────────────────

/// Every installed Bible.
#[tauri::command]
pub fn bibles_list(state: State<'_, AppState>) -> AppResult<Vec<BibleInfo>> {
    Ok(state.database().bibles().list()?)
}

/// Imports a Bible file (OSIS, Zefania, OpenSong, USX or USFM, or a zip of
/// them). Runs in the background; resolves when the Bible is ready.
#[tauri::command]
pub async fn bible_import(app: AppHandle, path: String) -> AppResult<BibleInfo> {
    let import = move || bibles::import_file(&app, &PathBuf::from(path));
    tauri::async_runtime::spawn_blocking(import).await?
}

/// Renames a Bible.
#[tauri::command]
pub fn bible_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: Id,
    name: String,
    abbreviation: String,
) -> AppResult<()> {
    let name = name.trim();
    let abbreviation = abbreviation.trim();
    state.database().bibles().rename(id, name, abbreviation)?;
    bibles::announce(&app);
    Ok(())
}

/// Deletes a Bible and its verses.
#[tauri::command]
pub fn bible_delete(app: AppHandle, state: State<'_, AppState>, id: Id) -> AppResult<()> {
    state.database().bibles().delete(id)?;
    bibles::announce(&app);
    Ok(())
}

/// Which books and chapters a Bible has, for the picker.
#[tauri::command]
pub fn bible_outline(state: State<'_, AppState>, id: Id) -> AppResult<Vec<BookOutline>> {
    Ok(state.database().bibles().outline(id)?)
}

/// Every verse of one chapter.
#[tauri::command]
pub fn bible_chapter(
    state: State<'_, AppState>,
    id: Id,
    book: BookId,
    chapter: u16,
) -> AppResult<Vec<Verse>> {
    let whole_chapter = VersePoint {
        chapter,
        verse: None,
    };
    let passage = Passage {
        book,
        ranges: vec![VerseRange {
            start: whole_chapter,
            end: whole_chapter,
        }],
    };
    Ok(state.database().bibles().passage(id, &passage)?)
}

/// Verses containing the given words.
#[tauri::command]
pub fn bible_search(state: State<'_, AppState>, id: Id, query: String) -> AppResult<Vec<Verse>> {
    Ok(state.database().bibles().search(id, &query)?)
}

// ── References ───────────────────────────────────────────────────────────

/// What the reference box should show for the text typed so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ReferenceFeedback {
    /// The understood passage, if the text is a reference.
    pub passage: Option<Passage>,
    /// The passage written out properly, e.g. "John 3:16–18".
    pub display: String,
    /// Other books a partly typed name could mean, e.g. ["Jude"] for "ju".
    pub alternatives: Vec<String>,
    /// Why the text is not a reference, if it is not.
    pub error: Option<String>,
}

/// Parses a reference as it is typed.
#[tauri::command]
pub fn scripture_parse(text: String) -> ReferenceFeedback {
    match reference::parse(&text) {
        Ok(parsed) => ReferenceFeedback {
            display: parsed.passage.to_string(),
            passage: Some(parsed.passage),
            alternatives: parsed.alternatives.iter().map(|book| book.name()).collect(),
            error: None,
        },
        Err(error) => ReferenceFeedback {
            passage: None,
            display: String::new(),
            alternatives: Vec::new(),
            error: Some(error.to_string()),
        },
    }
}

// ── Showing scripture ────────────────────────────────────────────────────

/// Loads a passage into the preview pane, in one or more translations
/// (shown side by side, in the order given).
#[tauri::command]
pub fn scripture_show_in_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    passage: Passage,
    bible_ids: Vec<Id>,
    options: ScriptureOptions,
) -> AppResult<()> {
    let translations = translations(&state, &passage, &bible_ids)?;
    dispatch(
        &app,
        &state,
        Command::PreviewScripture {
            passage,
            translations,
            options,
        },
    )
}

/// Adds a passage to the schedule at `index` (or at the end). The verse
/// text is copied into the schedule, so it travels with the file.
#[tauri::command]
pub fn schedule_add_scripture(
    app: AppHandle,
    state: State<'_, AppState>,
    passage: Passage,
    bible_ids: Vec<Id>,
    options: ScriptureOptions,
    index: Option<usize>,
) -> AppResult<()> {
    let translations = translations(&state, &passage, &bible_ids)?;
    let item = ScheduleItem::new(ItemContent::Scripture {
        reference: passage.to_string(),
        passage,
        translations,
        options,
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

/// A passage's verses in one Bible, e.g. to type into a presentation slide.
#[tauri::command]
pub fn scripture_text(
    state: State<'_, AppState>,
    passage: Passage,
    bible_id: Id,
) -> AppResult<TranslationText> {
    let mut found = translations(&state, &passage, &[bible_id])?;
    Ok(found.remove(0))
}

/// Looks up a passage in each chosen Bible.
fn translations(
    state: &AppState,
    passage: &Passage,
    bible_ids: &[Id],
) -> AppResult<Vec<TranslationText>> {
    if bible_ids.is_empty() {
        return Err(AppError::NoTranslation);
    }
    let database = state.database();
    let bibles = database.bibles();

    bible_ids
        .iter()
        .map(|&id| {
            let info = bibles.get(id)?.ok_or(AppError::BibleNotFound)?;
            Ok(TranslationText {
                bible_id: id,
                abbreviation: info.abbreviation,
                copyright: info.copyright,
                verses: bibles.passage(id, passage)?,
            })
        })
        .collect()
}
