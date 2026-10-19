//! Importing songs and Bibles from other worship software.
//!
//! The flow is: find sources ([`import_scan`]), read them and show the
//! operator what was found ([`import_preview`]), then save what they chose
//! ([`import_commit`]). On the very first launch, [`import_first_launch`]
//! scans the usual locations once, so a church moving from EasyWorship is
//! offered its library straight away.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sw_import::formats::{easyworship, openlp, read_file};
use sw_import::{ImportProblem, ImportReport, ImportedSong, duplicates};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::bibles;
use crate::error::AppResult;
use crate::import_sources::{self, FoundSource, SourceKind};
use crate::state::AppState;

/// Settings key recording that the first-launch scan has happened.
const FIRST_LAUNCH_KEY: &str = "importOffered";

/// A song found, and whether the library already has it.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct PreviewSong {
    /// The song.
    pub song: ImportedSong,
    /// The library already has a song with this title and lyrics.
    pub duplicate: bool,
}

/// A Bible found, ready to import.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct PreviewBible {
    /// The file.
    pub path: String,
    /// Its name, from the file (editable before importing).
    pub name: String,
    /// Its short name, from the file (editable before importing).
    pub abbreviation: String,
    /// How many verses it has.
    pub verse_count: u32,
    /// A Bible with this name is already installed.
    pub installed: bool,
}

/// Everything found in the chosen sources.
#[derive(Clone, Debug, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ImportPreview {
    /// The songs.
    pub songs: Vec<PreviewSong>,
    /// The Bibles.
    pub bibles: Vec<PreviewBible>,
    /// What could not be read.
    pub problems: Vec<ImportProblem>,
}

/// A Bible to import, with the name the operator settled on.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct BibleChoice {
    /// The file.
    pub path: String,
    /// The name to give it.
    pub name: String,
    /// The short name to give it.
    pub abbreviation: String,
}

/// What an import added.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct ImportSummary {
    /// How many songs were added.
    pub songs: u32,
    /// How many Bibles were added.
    pub bibles: u32,
    /// Bibles that failed to import.
    pub problems: Vec<ImportProblem>,
}

/// Looks for importable libraries: in `folder` if given, otherwise in the
/// places other worship programs keep their data.
#[tauri::command]
pub async fn import_scan(folder: Option<String>) -> Vec<FoundSource> {
    let scan = move || match folder {
        Some(folder) => import_sources::scan_folder(Path::new(&folder)),
        None => import_sources::scan_default_locations(),
    };
    tauri::async_runtime::spawn_blocking(scan)
        .await
        .unwrap_or_default()
}

/// The first time SimpleWorship runs, scans the usual locations and
/// returns what it found (often nothing). Later calls return nothing, so
/// the operator is only asked once; File › Import from Other Software…
/// is always there afterwards.
#[tauri::command]
pub async fn import_first_launch(app: AppHandle) -> AppResult<Vec<FoundSource>> {
    let state = app.state::<AppState>();
    let offered: Option<bool> = state.database().settings().get(FIRST_LAUNCH_KEY)?;
    if offered == Some(true) {
        return Ok(Vec::new());
    }
    state.database().settings().set(FIRST_LAUNCH_KEY, &true)?;
    Ok(import_scan(None).await)
}

/// Reads the chosen sources and reports what is in them, marking songs the
/// library already has.
#[tauri::command]
pub async fn import_preview(app: AppHandle, sources: Vec<FoundSource>) -> AppResult<ImportPreview> {
    let read = move || -> AppResult<ImportPreview> {
        let state = app.state::<AppState>();
        let mut report = ImportReport::default();
        let mut bible_paths = Vec::new();

        for source in &sources {
            match source.kind {
                SourceKind::EasyWorshipSongs => {
                    report.extend(easyworship::read_library(Path::new(&source.path)));
                }
                SourceKind::OpenLpSongs => {
                    report.extend(openlp::read_library(Path::new(&source.path)))
                }
                SourceKind::SongFiles => {
                    for file in &source.files {
                        report.extend(read_file(Path::new(file)));
                    }
                }
                SourceKind::EasyWorshipBible => bible_paths.push(PathBuf::from(&source.path)),
            }
        }

        let known: HashSet<String> = state
            .database()
            .songs()
            .all()?
            .iter()
            .map(|song| duplicates::key(&song.title, &song.lyrics))
            .collect();
        // Songs repeated within the sources themselves count as duplicates too.
        let mut seen = known;
        let songs = std::mem::take(&mut report.songs)
            .into_iter()
            .map(|song| {
                let fresh = seen.insert(duplicates::key(&song.title, &song.lyrics));
                PreviewSong {
                    song,
                    duplicate: !fresh,
                }
            })
            .collect();

        let installed: Vec<String> = state
            .database()
            .bibles()
            .list()?
            .into_iter()
            .map(|bible| bible.name.to_lowercase())
            .collect();
        let mut preview_bibles = Vec::new();
        for path in bible_paths {
            match bibles::read(&path) {
                Ok(bible) => preview_bibles.push(PreviewBible {
                    path: path.display().to_string(),
                    installed: installed.contains(&bible.name.to_lowercase()),
                    verse_count: bible.verses.len() as u32,
                    name: bible.name,
                    abbreviation: bible.abbreviation,
                }),
                Err(error) => report.problem(path.display().to_string(), error.to_string()),
            }
        }

        Ok(ImportPreview {
            songs,
            bibles: preview_bibles,
            problems: report.problems,
        })
    };
    tauri::async_runtime::spawn_blocking(read).await?
}

/// Saves the chosen songs (in one transaction) and imports the chosen
/// Bibles. Runs in the background.
#[tauri::command]
pub async fn import_commit(
    app: AppHandle,
    songs: Vec<ImportedSong>,
    bibles: Vec<BibleChoice>,
) -> AppResult<ImportSummary> {
    let commit = move || -> AppResult<ImportSummary> {
        let state = app.state::<AppState>();
        let songs: Vec<_> = songs.into_iter().map(ImportedSong::into_song).collect();
        let saved = state.database().songs().save_all(&songs)?;

        let mut summary = ImportSummary {
            songs: saved as u32,
            bibles: 0,
            problems: Vec::new(),
        };
        for choice in bibles {
            let path = Path::new(&choice.path);
            match bibles::import_file_as(&app, path, Some(&choice.name), Some(&choice.abbreviation))
            {
                Ok(_) => summary.bibles += 1,
                Err(error) => summary.problems.push(ImportProblem {
                    source: choice.name,
                    message: error.to_string(),
                }),
            }
        }
        Ok(summary)
    };
    tauri::async_runtime::spawn_blocking(commit).await?
}
