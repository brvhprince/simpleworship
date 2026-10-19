//! Finding song libraries and Bibles from other worship software.
//!
//! On first launch (and from File › Import from Other Software…) the app
//! looks in the places these programs keep their data:
//!
//! | Program        | Where                                                      |
//! | -------------- | ---------------------------------------------------------- |
//! | EasyWorship    | `%PUBLIC%\Documents\Softouch\EasyWorship` (songs) and      |
//! |                | `%ProgramData%\Softouch\EasyWorship` (Bibles), Windows only |
//! | OpenLP         | its data folder, on Windows, macOS and Linux               |
//! | ProPresenter 6 | `Documents/ProPresenter6` (`.pro6` files)                  |
//!
//! Any other folder can be searched too, for example a copy of an
//! EasyWorship installation brought over from a Windows computer.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sw_import::formats::{easyworship, looks_like_song_file};
use ts_rs::TS;

/// How deep to look for loose song files inside a chosen folder.
const SONG_FILE_DEPTH: usize = 3;

/// The kind of thing that was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub enum SourceKind {
    /// An EasyWorship `Data` folder.
    EasyWorshipSongs,
    /// An EasyWorship Bible (`.ewb`).
    EasyWorshipBible,
    /// An OpenLP `songs.sqlite`.
    OpenLpSongs,
    /// Song files (`.pro6`, OpenLyrics, ChordPro, SongSelect, text).
    SongFiles,
}

/// Something that can be imported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct FoundSource {
    /// What it is.
    pub kind: SourceKind,
    /// A name to show, e.g. "EasyWorship songs" or "KJV.ewb".
    pub label: String,
    /// Where it is: a folder, a database, or a Bible file.
    pub path: String,
    /// For [`SourceKind::SongFiles`], the files themselves.
    pub files: Vec<String>,
}

/// Searches the usual places for every supported program.
pub fn scan_default_locations() -> Vec<FoundSource> {
    let mut found = Vec::new();
    for folder in easyworship_song_roots() {
        found.extend(scan_easyworship(&folder));
    }
    for folder in easyworship_bible_roots() {
        found.extend(bibles_in(&folder));
    }
    for database in openlp_databases() {
        if database.is_file() {
            found.push(openlp(&database));
        }
    }
    for folder in propresenter_folders() {
        found.extend(song_files_in(&folder, "ProPresenter 6 songs"));
    }
    dedupe(found)
}

/// Searches a folder the operator chose, for anything importable.
pub fn scan_folder(folder: &Path) -> Vec<FoundSource> {
    let mut found = scan_easyworship(folder);
    found.extend(bibles_in(folder));
    if let Some(database) = find_file(folder, "songs.sqlite", 4) {
        found.push(openlp(&database));
    }
    found.extend(song_files_in(folder, "Song files"));
    dedupe(found)
}

fn scan_easyworship(folder: &Path) -> Vec<FoundSource> {
    easyworship::find_data_folder(folder)
        .map(|data| FoundSource {
            kind: SourceKind::EasyWorshipSongs,
            label: "EasyWorship songs".into(),
            path: data.display().to_string(),
            files: Vec::new(),
        })
        .into_iter()
        .collect()
}

fn bibles_in(folder: &Path) -> Vec<FoundSource> {
    easyworship::find_bibles(folder)
        .into_iter()
        .map(|path| FoundSource {
            kind: SourceKind::EasyWorshipBible,
            label: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.display().to_string(),
            files: Vec::new(),
        })
        .collect()
}

fn openlp(database: &Path) -> FoundSource {
    FoundSource {
        kind: SourceKind::OpenLpSongs,
        label: "OpenLP songs".into(),
        path: database.display().to_string(),
        files: Vec::new(),
    }
}

fn song_files_in(folder: &Path, label: &str) -> Vec<FoundSource> {
    let mut files = Vec::new();
    collect_song_files(folder, SONG_FILE_DEPTH, &mut files);
    if files.is_empty() {
        return Vec::new();
    }
    files.sort();
    vec![FoundSource {
        kind: SourceKind::SongFiles,
        label: format!("{label} ({})", files.len()),
        path: folder.display().to_string(),
        files: files.iter().map(|f| f.display().to_string()).collect(),
    }]
}

fn collect_song_files(folder: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if path.is_dir() {
            // An EasyWorship installation copied whole has thousands of
            // non-song files; its songs are imported from the database.
            if depth > 0 && !name.starts_with('.') && name != "data" && name != "plugins" {
                collect_song_files(&path, depth - 1, found);
            }
        } else if !name.starts_with('.') && looks_like_song_file(&path) {
            found.push(path);
        }
    }
}

fn find_file(folder: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    let candidate = folder.join(name);
    if candidate.is_file() {
        return Some(candidate);
    }
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .find_map(|child| find_file(&child, name, depth - 1))
}

/// The same thing can be reachable from two starting points.
fn dedupe(found: Vec<FoundSource>) -> Vec<FoundSource> {
    let mut unique: Vec<FoundSource> = Vec::new();
    for source in found {
        if !unique
            .iter()
            .any(|known| known.path == source.path && known.kind == source.kind)
        {
            unique.push(source);
        }
    }
    unique
}

// ── Where each program keeps its data ────────────────────────────────────

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn home() -> Option<PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}

fn easyworship_song_roots() -> Vec<PathBuf> {
    let public = env_path("PUBLIC").unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"));
    let root = public
        .join("Documents")
        .join("Softouch")
        .join("EasyWorship");
    if cfg!(windows) && root.is_dir() {
        vec![root]
    } else {
        Vec::new()
    }
}

fn easyworship_bible_roots() -> Vec<PathBuf> {
    let program_data = env_path("ProgramData").unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let root = program_data.join("Softouch").join("EasyWorship");
    if cfg!(windows) && root.is_dir() {
        vec![root]
    } else {
        Vec::new()
    }
}

fn openlp_databases() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(app_data) = env_path("APPDATA") {
        candidates.push(
            app_data
                .join("openlp")
                .join("data")
                .join("songs")
                .join("songs.sqlite"),
        );
    }
    if let Some(home) = home() {
        candidates.push(home.join("Library/Application Support/openlp/Data/songs/songs.sqlite"));
        candidates.push(home.join(".local/share/openlp/songs/songs.sqlite"));
    }
    candidates
}

fn propresenter_folders() -> Vec<PathBuf> {
    home()
        .map(|home| home.join("Documents").join("ProPresenter6"))
        .filter(|folder| folder.is_dir())
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scans a real folder given in `SW_IMPORT_FOLDER` and prints what it
    /// finds. Real libraries are never committed, so this is opt-in:
    /// `SW_IMPORT_FOLDER=/path cargo test -p simpleworship-desktop scan_real -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real folder; see the docs"]
    fn scan_real_folder() {
        let Ok(folder) = std::env::var("SW_IMPORT_FOLDER") else {
            return;
        };
        for source in scan_folder(Path::new(&folder)) {
            println!(
                "{:?} · {} · {} files",
                source.kind,
                source.label,
                source.files.len()
            );
        }
    }
}
