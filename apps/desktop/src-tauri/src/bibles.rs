//! Bible imports that run in the background: the Bibles bundled with the
//! app (on first launch) and files the operator imports.
//!
//! An import parses tens of thousands of verses and writes them in one
//! transaction, which takes a few seconds. It runs on its own thread with
//! its own database connection, so the operator window stays responsive.
//! When it finishes, every window hears [`BIBLES_EVENT`] and reloads its
//! list of Bibles.

use std::path::Path;

use sw_bible::import::{self, BibleData};
use sw_core::BibleInfo;
use sw_db::Database;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::AppResult;
use crate::state::AppState;

/// Broadcast whenever Bibles are added, renamed or removed.
/// Keep in sync with `apps/desktop/src/lib/bibles.svelte.ts`.
pub const BIBLES_EVENT: &str = "bibles://changed";

/// A Bible that ships with the app: its key, resource file and display name.
struct Bundled {
    key: &'static str,
    file: &'static str,
    name: &'static str,
    abbreviation: &'static str,
}

/// See `resources/bibles/README.md` for sources and licences.
const BUNDLED: &[Bundled] = &[
    Bundled {
        key: "kjv",
        file: "bibles/eng-kjv2006_usfm.zip",
        name: "King James Version",
        abbreviation: "KJV",
    },
    Bundled {
        key: "web",
        file: "bibles/engwebp_usfm.zip",
        name: "World English Bible",
        abbreviation: "WEB",
    },
];

/// Imports any bundled Bible that has never been imported, on a background
/// thread. A Bible the operator deleted is not brought back.
pub fn install_bundled(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(error) = install_missing_bundled(&app) {
            log::warn!("could not install the bundled Bibles: {error}");
        }
    });
}

fn install_missing_bundled(app: &AppHandle) -> AppResult<()> {
    let database = Database::open(&app.state::<AppState>().database_path)?;

    for bundled in BUNDLED {
        if database.bibles().has_bundled(bundled.key)? {
            continue;
        }
        let path = app.path().resolve(bundled.file, BaseDirectory::Resource)?;
        let mut bible = read(&path)?;
        bible.name = bundled.name.to_owned();
        bible.abbreviation = bundled.abbreviation.to_owned();

        database.bibles().import(&bible, Some(bundled.key))?;
        announce(app);
    }
    Ok(())
}

/// Imports a Bible file chosen by the operator. Blocks until done, so call
/// it from a background task.
pub fn import_file(app: &AppHandle, path: &Path) -> AppResult<BibleInfo> {
    import_file_as(app, path, None, None)
}

/// Like [`import_file`], with a name and abbreviation to use instead of the
/// file's own (some files have wrong ones).
pub fn import_file_as(
    app: &AppHandle,
    path: &Path,
    name: Option<&str>,
    abbreviation: Option<&str>,
) -> AppResult<BibleInfo> {
    let mut bible = read(path)?;
    if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
        name.trim().clone_into(&mut bible.name);
    }
    if let Some(abbreviation) = abbreviation.filter(|abbreviation| !abbreviation.trim().is_empty())
    {
        abbreviation.trim().clone_into(&mut bible.abbreviation);
    }
    let database = Database::open(&app.state::<AppState>().database_path)?;
    let info = database.bibles().import(&bible, None)?;
    announce(app);
    Ok(info)
}

/// Reads any supported Bible file, including EasyWorship `.ewb`.
pub fn read(path: &Path) -> AppResult<BibleData> {
    Ok(import::import_file(path)?)
}

/// Tells every window that the list of Bibles changed.
pub fn announce(app: &AppHandle) {
    if let Err(error) = app.emit(BIBLES_EVENT, ()) {
        log::warn!("could not announce Bible changes: {error}");
    }
}
