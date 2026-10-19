//! The media library's files: importing (copying into the library folder),
//! reading basic details, and deleting.
//!
//! Copying a large video takes a while, so imports run on a background
//! thread and report progress on [`MEDIA_PROGRESS_EVENT`]. When the
//! library changes, every window hears [`MEDIA_EVENT`].

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sw_core::time;
use sw_core::{Id, MediaItem, MediaKind, MediaRef};
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::state::{AppState, MediaFolders};

/// Broadcast whenever media is added, renamed or removed.
/// Keep in sync with `apps/desktop/src/lib/media.svelte.ts`.
pub const MEDIA_EVENT: &str = "media://changed";

/// Broadcast while a media file is being copied.
pub const MEDIA_PROGRESS_EVENT: &str = "media://progress";

/// Progress of a media import, for the progress bar.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "desktop.ts")]
pub struct MediaProgress {
    /// The file being copied.
    pub name: String,
    /// Bytes copied so far.
    #[ts(type = "number")]
    pub copied: u64,
    /// The file's size.
    #[ts(type = "number")]
    pub total: u64,
}

/// File extensions we import, and what they are.
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "avif"];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "webm"];
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "aac", "wav", "ogg", "oga", "flac"];

/// Copy in chunks this big, announcing progress after each one.
const CHUNK: usize = 4 * 1024 * 1024;

/// What a file is, judged by its extension.
pub fn kind_of(path: &Path) -> Option<MediaKind> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else if AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Audio)
    } else {
        None
    }
}

/// The full path of a media file.
pub fn file_path(folders: &MediaFolders, media: &MediaRef) -> PathBuf {
    folders.media.join(&media.file)
}

/// Copies `source` into the library folder and records it. Blocks until
/// done, so call it from a background task.
pub fn import_file(app: &AppHandle, source: &Path) -> AppResult<MediaItem> {
    let kind = kind_of(source).ok_or(AppError::UnsupportedMedia)?;
    let state = app.state::<AppState>();

    let id = Id::new();
    let extension = source
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let file = format!("{id}.{extension}");
    let name = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.clone());

    let destination = state.media_folders.media.join(&file);
    let size_bytes = copy_with_progress(app, source, &destination, &name)?;

    let (width, height) = match kind {
        // Videos are measured by the webview later (see `media_update_details`);
        // audio has no picture.
        MediaKind::Video | MediaKind::Audio => (None, None),
        MediaKind::Image => imagesize::size(&destination)
            .map(|size| {
                (
                    u32::try_from(size.width).ok(),
                    u32::try_from(size.height).ok(),
                )
            })
            .unwrap_or((None, None)),
    };

    let item = MediaItem {
        id,
        kind,
        name,
        file,
        width,
        height,
        duration_ms: None,
        size_bytes,
        created_at: time::now(),
        collection: None,
    };
    state.database().media().save(&item)?;
    announce(app);
    Ok(item)
}

/// Copies a file in chunks, announcing progress. A partly copied file is
/// removed if anything goes wrong.
fn copy_with_progress(app: &AppHandle, from: &Path, to: &Path, name: &str) -> AppResult<u64> {
    let result = (|| -> io::Result<u64> {
        let total = fs::metadata(from)?.len();
        let mut reader = BufReader::new(File::open(from)?);
        let mut writer = BufWriter::new(File::create(to)?);
        let mut buffer = vec![0; CHUNK];
        let mut copied = 0;

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            writer.write_all(&buffer[..read])?;
            copied += read as u64;
            let progress = MediaProgress {
                name: name.to_owned(),
                copied,
                total,
            };
            let _ = app.emit(MEDIA_PROGRESS_EVENT, progress);
        }
        writer.flush()?;
        Ok(copied)
    })();

    if result.is_err() {
        let _ = fs::remove_file(to);
    }
    Ok(result?)
}

/// Removes a media item's file and thumbnail. Missing files are fine.
pub fn remove_files(folders: &MediaFolders, item: &MediaItem) {
    let _ = fs::remove_file(folders.media.join(&item.file));
    let _ = fs::remove_file(thumbnail_path(folders, item.id));
}

/// Where a media item's thumbnail is kept.
pub fn thumbnail_path(folders: &MediaFolders, id: Id) -> PathBuf {
    folders.thumbnails.join(format!("{id}.jpg"))
}

/// Tells every window that the media library changed.
pub fn announce(app: &AppHandle) {
    if let Err(error) = app.emit(MEDIA_EVENT, ()) {
        eprintln!("could not announce media changes: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_media_by_extension() {
        assert_eq!(kind_of(Path::new("/a/Loop.MP4")), Some(MediaKind::Video));
        assert_eq!(kind_of(Path::new("logo.png")), Some(MediaKind::Image));
        assert_eq!(kind_of(Path::new("Hymn.M4A")), Some(MediaKind::Audio));
        assert_eq!(kind_of(Path::new("notes.txt")), None);
        assert_eq!(kind_of(Path::new("no-extension")), None);
    }
}
