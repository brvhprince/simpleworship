//! Background audio: a playlist that plays alongside the slides (music
//! before the service, during communion…), independent of what is on screen.
//!
//! The operator's computer plays it, in the operator window, so it works
//! even with no projector window open. Like video, playback is an anchor
//! plus a clock, so the operator UI can show the position without the
//! engine ticking.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::media::MediaItem;
use crate::time::Timestamp;

/// A fade-out in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Fade {
    /// When the fade began.
    #[ts(type = "number")]
    pub started_at: Timestamp,
    /// How long it takes, in milliseconds. Playback stops at the end.
    #[ts(type = "number")]
    pub duration_ms: u64,
}

/// The background audio player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct AudioState {
    /// The tracks, in playing order.
    pub playlist: Vec<MediaItem>,
    /// The current track.
    pub index: usize,
    /// Whether it is playing.
    pub playing: bool,
    /// Position in the current track at `anchor`, in milliseconds.
    #[ts(type = "number")]
    pub position_ms: u64,
    /// When `position_ms` was measured.
    #[ts(type = "number")]
    pub anchor: Timestamp,
    /// Volume, from 0 (silent) to 1 (full).
    pub volume: f32,
    /// Start the playlist again after the last track.
    #[serde(rename = "loop")]
    pub looping: bool,
    /// A fade-out in progress, if any.
    pub fade: Option<Fade>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            playlist: Vec::new(),
            index: 0,
            playing: false,
            position_ms: 0,
            anchor: 0,
            volume: 0.8,
            looping: false,
            fade: None,
        }
    }
}

impl AudioState {
    /// The current track, if there is one.
    pub fn current(&self) -> Option<&MediaItem> {
        self.playlist.get(self.index)
    }

    /// Position in the current track at `now`.
    pub fn position_at(&self, now: Timestamp) -> u64 {
        let since = if self.playing {
            u64::try_from(now - self.anchor).unwrap_or(0)
        } else {
            0
        };
        self.position_ms + since
    }

    /// Starts track `index` from the beginning.
    pub(crate) fn start_track(&mut self, index: usize, now: Timestamp) {
        self.index = index;
        self.position_ms = 0;
        self.anchor = now;
        self.playing = true;
        self.fade = None;
    }

    /// The track after the current one, or `None` at the end of the
    /// playlist (unless it loops).
    pub(crate) fn next_index(&self) -> Option<usize> {
        let next = self.index + 1;
        if next < self.playlist.len() {
            Some(next)
        } else if self.looping && !self.playlist.is_empty() {
            Some(0)
        } else {
            None
        }
    }
}
