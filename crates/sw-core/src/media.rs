//! Media: the images and videos in the library.
//!
//! Media files live in the library's own folder, copied there on import, so
//! a video can never go "offline" mid-service because a USB stick was
//! removed. Everything else refers to a file by [`MediaRef`]: its id and
//! its file name inside that folder. The host turns a `MediaRef` into a URL
//! the webview can load; a saved schedule carries the files it refers to.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::time::Timestamp;

/// What kind of file a media item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum MediaKind {
    /// A still image (PNG, JPEG, WebP, GIF…).
    Image,
    /// A video (MP4 with H.264 is the most widely playable).
    Video,
    /// A sound file (MP3, M4A, WAV…), for background music.
    Audio,
}

/// A pointer to a file in the media library.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct MediaRef {
    /// The media item's id.
    pub id: Id,
    /// Its file name inside the library's media folder, e.g. `"0192….mp4"`.
    pub file: String,
}

/// An image or video in the media library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct MediaItem {
    /// Unique identifier.
    pub id: Id,
    /// Image or video.
    pub kind: MediaKind,
    /// The name shown in the library, usually the original file name
    /// without its extension.
    pub name: String,
    /// The file name inside the library's media folder.
    pub file: String,
    /// Width in pixels, if known.
    pub width: Option<u32>,
    /// Height in pixels, if known.
    pub height: Option<u32>,
    /// Length of a video in milliseconds, if known.
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    /// File size in bytes.
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// When it was imported.
    #[ts(type = "number")]
    pub created_at: Timestamp,
    /// The presentation this is a page of, if any. Pages are kept out of
    /// the Media tab and shown in the Presentations tab instead.
    #[serde(default)]
    pub collection: Option<Id>,
}

impl MediaItem {
    /// The pointer other records use to refer to this item.
    pub fn as_ref(&self) -> MediaRef {
        MediaRef {
            id: self.id,
            file: self.file.clone(),
        }
    }
}
