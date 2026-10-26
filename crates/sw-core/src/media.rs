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
use crate::theme::MediaFit;
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

/// How a picture or video fills the screen, chosen for the file itself in
/// the Media tab and used wherever the file fills the screen: as a theme's
/// or a slide's background, or shown on its own. (A picture placed on a
/// slide in the canvas editor has its own fit, set in the Inspector.)
///
/// As in EasyWorship, most files are left on `Automatic`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub enum MediaScaling {
    /// Do what the place it is used asks for: a theme background as the
    /// theme's designer set it, a picture on its own with black bars.
    #[default]
    Automatic,
    /// Stretch it to the screen's shape, distorting it if needed.
    Stretch,
    /// Show all of it, with black bars if its shape differs.
    Maintain,
    /// Fill the screen, cropping the edges if its shape differs.
    Zoom,
}

impl MediaScaling {
    /// The fit to use instead of the place's own, or `None` for automatic.
    pub fn fit(self) -> Option<MediaFit> {
        match self {
            Self::Automatic => None,
            Self::Stretch => Some(MediaFit::Stretch),
            Self::Maintain => Some(MediaFit::Contain),
            Self::Zoom => Some(MediaFit::Cover),
        }
    }
}

/// The files that are not on [`MediaScaling::Automatic`], and the fit each
/// one asks for. The host keeps the engine's copy up to date (see
/// [`Command::SetFileFits`](crate::Command::SetFileFits)).
pub type FileFits = std::collections::BTreeMap<Id, MediaFit>;

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
    /// How it fills the screen (see [`MediaScaling`]).
    #[serde(default)]
    pub scaling: MediaScaling,
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

/// The church logo, shown by the Logo button: a picture, or a video that
/// plays on a loop, without sound.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Logo {
    /// The file.
    pub media: MediaRef,
    /// Picture or video (never sound).
    pub kind: MediaKind,
}

impl Logo {
    /// The logo for `item`, or `None` for a sound file, which has no picture.
    pub fn of(item: &MediaItem) -> Option<Self> {
        (item.kind != MediaKind::Audio).then(|| Self {
            media: item.as_ref(),
            kind: item.kind,
        })
    }
}

/// A camera or capture card connected to the computer, as a live picture
/// (see the Cameras section of the Media tab).
///
/// Each window opens the camera itself, finding it by `device_id` (the
/// system's identifier, which can change, e.g. on another computer), else
/// by `label` (its name, such as "Elgato HD60 S"), so a saved schedule
/// still finds the same model of camera.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct CameraRef {
    /// The system's identifier for the camera.
    pub device_id: String,
    /// The camera's name.
    pub label: String,
}
