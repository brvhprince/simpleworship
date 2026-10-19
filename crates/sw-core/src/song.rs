//! Songs in the library.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::lyrics::{self, Section};
use crate::time::Timestamp;

/// A song in the library.
///
/// Lyrics are kept as tagged plain text (see [`crate::lyrics`]) so the editor
/// shows exactly what was saved, and full-text search can index them as-is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Song {
    /// Unique identifier.
    pub id: Id,
    /// Song title, e.g. "Amazing Grace".
    pub title: String,
    /// Author(s) / songwriter(s), free text.
    pub author: String,
    /// Copyright notice, e.g. "© 1779 Public Domain".
    pub copyright: String,
    /// CCLI song number, used for licence reporting. Empty if unknown.
    pub ccli_number: String,
    /// Tagged lyric text.
    pub lyrics: String,
    /// Optional section order, e.g. `"V1 C V2 C B C"`. Empty means document order.
    pub arrangement: String,
    /// The song's own theme, if it should not use the default song theme.
    #[serde(default)]
    pub theme_id: Option<Id>,
    /// When the song was first saved.
    #[ts(type = "number")]
    pub created_at: Timestamp,
    /// When the song was last saved.
    #[ts(type = "number")]
    pub updated_at: Timestamp,
}

impl Song {
    /// Creates an empty, unsaved song with a fresh id.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            title: title.into(),
            author: String::new(),
            copyright: String::new(),
            ccli_number: String::new(),
            lyrics: String::new(),
            arrangement: String::new(),
            theme_id: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    /// The song's sections in display order: the arrangement if one is set,
    /// otherwise the order they appear in the lyrics.
    pub fn ordered_sections(&self) -> Vec<Section> {
        let sections = lyrics::parse_sections(&self.lyrics);
        let arrangement = lyrics::parse_arrangement(&self.arrangement);
        lyrics::arrange(&sections, &arrangement)
    }

    /// The footer line shown under the lyrics: copyright and CCLI number.
    /// Returns `None` when there is nothing to show.
    pub fn footer(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.author.trim().is_empty() {
            parts.push(self.author.trim().to_owned());
        }
        if !self.copyright.trim().is_empty() {
            parts.push(self.copyright.trim().to_owned());
        }
        if !self.ccli_number.trim().is_empty() {
            parts.push(format!("CCLI Song # {}", self.ccli_number.trim()));
        }
        (!parts.is_empty()).then(|| parts.join(" · "))
    }
}

/// A lightweight row for song lists and search results.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct SongSummary {
    /// The song's id.
    pub id: Id,
    /// Song title.
    pub title: String,
    /// Author(s).
    pub author: String,
    /// A line of context: the matching lyric excerpt for a search, or the
    /// first lyric line when listing.
    pub excerpt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_joins_the_parts_that_exist() {
        let mut song = Song::new("Amazing Grace");
        assert_eq!(song.footer(), None);

        song.author = "John Newton".into();
        song.ccli_number = "22025".into();
        assert_eq!(song.footer().unwrap(), "John Newton · CCLI Song # 22025");
    }

    #[test]
    fn ordered_sections_follow_the_arrangement() {
        let mut song = Song::new("Test");
        song.lyrics = "[V1]\nOne\n[C]\nChorus".into();
        song.arrangement = "C V1 C".into();

        let texts: Vec<_> = song
            .ordered_sections()
            .into_iter()
            .map(|s| s.text)
            .collect();
        assert_eq!(texts, ["Chorus", "One", "Chorus"]);
    }
}
