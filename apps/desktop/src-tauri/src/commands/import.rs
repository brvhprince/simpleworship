//! Importing songs, Bibles, and pictures and videos from other worship
//! software.
//!
//! The flow is: find sources ([`import_scan`]), read them and show the
//! operator what was found ([`import_preview`]), then save what they chose
//! ([`import_commit`]). On the very first launch, [`import_first_launch`]
//! scans the usual locations once, so a church moving from EasyWorship is
//! offered its library straight away.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sw_core::MediaKind;
use sw_import::formats::easyworship_media::{self, EasyWorshipMedia};
use sw_import::formats::{easyworship, openlp, read_file};
use sw_import::{ImportProblem, ImportReport, ImportedSong, duplicates};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::error::AppResult;
use crate::import_sources::{self, FoundSource, SourceKind};
use crate::state::AppState;
use crate::{bibles, media};

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

/// A picture, video or sound file from another program's media library.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct PreviewMedia {
    /// Its title in the other program.
    pub title: String,
    /// The file's name.
    pub file_name: String,
    /// The `Resources` subfolder it should be in, e.g. "Videos".
    pub folder: Option<String>,
    /// The library's `Data` folder, to search from again.
    pub data_folder: String,
    /// Where the file was found, if it was.
    pub path: Option<String>,
    /// What it is, or `None` for a format SimpleWorship cannot show (e.g. WMV).
    pub kind: Option<MediaKind>,
    /// The media library already has it (same title and size).
    pub duplicate: bool,
    /// The `Resources` folder searched, or `None` if none was found near the
    /// library (shown when files are missing, to see where it looked).
    pub searched: Option<String>,
}

/// A media file to import, with the title to give it.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct MediaChoice {
    /// The file.
    pub path: String,
    /// Its title.
    pub title: String,
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
    /// Pictures, videos and sound from media libraries.
    pub media: Vec<PreviewMedia>,
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
    /// How many pictures, videos and sound files were added.
    #[serde(default)]
    pub media: u32,
    /// Bibles and media that failed to import.
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
        let mut media = Vec::new();

        for source in &sources {
            match source.kind {
                SourceKind::EasyWorshipSongs => {
                    let data = Path::new(&source.path);
                    report.extend(easyworship::read_library(data));
                    match easyworship_media::read_media(data) {
                        Ok(found) => media.extend(preview_media(data, &found, &[])),
                        Err(message) => report.problem(source.path.clone(), message),
                    }
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

        mark_known_media(&state, &mut media)?;
        Ok(ImportPreview {
            songs,
            bibles: preview_bibles,
            media,
            problems: report.problems,
        })
    };
    tauri::async_runtime::spawn_blocking(read).await?
}

/// Looks for media files that were not found, in `folder` (and below), e.g.
/// a copy of EasyWorship's `Resources` folder from another computer.
#[tauri::command]
pub async fn import_locate_media(
    app: AppHandle,
    media: Vec<PreviewMedia>,
    folder: String,
) -> AppResult<Vec<PreviewMedia>> {
    let locate = move || -> AppResult<Vec<PreviewMedia>> {
        let state = app.state::<AppState>();
        let extra = [PathBuf::from(&folder)];
        let mut updated: Vec<PreviewMedia> = media
            .into_iter()
            .map(|item| {
                if item.path.is_some() {
                    return item;
                }
                let wanted = EasyWorshipMedia {
                    title: item.title.clone(),
                    file_name: item.file_name.clone(),
                    folder: item.folder.clone(),
                    absolute_folder: None,
                };
                let data = PathBuf::from(&item.data_folder);
                preview_media(&data, &[wanted], &extra)
                    .pop()
                    .unwrap_or(item)
            })
            .collect();
        mark_known_media(&state, &mut updated)?;
        Ok(updated)
    };
    tauri::async_runtime::spawn_blocking(locate).await?
}

/// Media entries for a library's items, with the files found so far.
fn preview_media(data: &Path, items: &[EasyWorshipMedia], extra: &[PathBuf]) -> Vec<PreviewMedia> {
    let found = easyworship_media::locate(data, items, extra);
    let searched = easyworship_media::resources_folder(data).map(|path| path.display().to_string());
    log::info!(
        "EasyWorship media: {} of {} files found; Resources folder: {}",
        found.iter().flatten().count(),
        items.len(),
        searched.as_deref().unwrap_or("not found"),
    );
    items
        .iter()
        .zip(found)
        .map(|(item, path)| PreviewMedia {
            title: item.title.clone(),
            file_name: item.file_name.clone(),
            folder: item.folder.clone(),
            data_folder: data.display().to_string(),
            kind: media::kind_of(Path::new(&item.file_name)),
            path: path.map(|path| path.display().to_string()),
            duplicate: false,
            searched: searched.clone(),
        })
        .collect()
}

/// Marks media the library already has: the same title and file size.
fn mark_known_media(state: &AppState, media: &mut [PreviewMedia]) -> AppResult<()> {
    let known: HashSet<(String, u64)> = state
        .database()
        .media()
        .list()?
        .into_iter()
        .map(|item| (item.name.to_lowercase(), item.size_bytes))
        .collect();
    for item in media {
        let size = item
            .path
            .as_ref()
            .and_then(|path| std::fs::metadata(path).ok())
            .map(|metadata| metadata.len());
        item.duplicate =
            size.is_some_and(|size| known.contains(&(item.title.to_lowercase(), size)));
    }
    Ok(())
}

/// Saves the chosen songs (in one transaction), imports the chosen Bibles,
/// and copies the chosen media into the media library. Runs in the
/// background.
#[tauri::command]
pub async fn import_commit(
    app: AppHandle,
    songs: Vec<ImportedSong>,
    bibles: Vec<BibleChoice>,
    media: Option<Vec<MediaChoice>>,
) -> AppResult<ImportSummary> {
    let commit = move || -> AppResult<ImportSummary> {
        let state = app.state::<AppState>();
        let songs: Vec<_> = songs.into_iter().map(ImportedSong::into_song).collect();
        let saved = state.database().songs().save_all(&songs)?;
        if saved > 0 {
            crate::remote::share(&app, crate::commands::songs::SONGS_EVENT, ());
        }

        let mut summary = ImportSummary {
            songs: saved as u32,
            bibles: 0,
            media: 0,
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
        for choice in media.unwrap_or_default() {
            match import_media_file(&app, &choice) {
                Ok(()) => summary.media += 1,
                Err(error) => summary.problems.push(ImportProblem {
                    source: choice.title,
                    message: error.to_string(),
                }),
            }
        }
        Ok(summary)
    };
    tauri::async_runtime::spawn_blocking(commit).await?
}

/// Copies one media file into the library, titled as in the other program.
fn import_media_file(app: &AppHandle, choice: &MediaChoice) -> AppResult<()> {
    let mut item = media::import_file(app, Path::new(&choice.path))?;
    if !choice.title.trim().is_empty() {
        item.name = choice.title.trim().to_owned();
        app.state::<AppState>().database().media().save(&item)?;
    }
    media::announce(app);
    Ok(())
}
