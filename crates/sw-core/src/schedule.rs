//! Schedules: the order of service.
//!
//! A [`Schedule`] is an ordered list of [`ScheduleItem`]s: the songs and
//! other content for one service. Each item carries **its own copy** of its
//! content (a song item contains the whole song), for two reasons:
//!
//! - A saved schedule opens on another computer that does not have the
//!   same songs in its library.
//! - The engine can turn any item into slides without asking the database,
//!   which keeps this crate free of I/O.
//!
//! When a song is edited in the library, the host sends
//! [`Command::RefreshSong`](crate::Command::RefreshSong) so the copies in the
//! schedule stay up to date.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::id::Id;
use crate::presentation::Presentation;
use crate::slides::SlideOptions;
use crate::song::Song;
use crate::theme::Theme;

/// The order of service.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Schedule {
    /// The items, in service order.
    pub items: Vec<ScheduleItem>,
}

/// One entry in a schedule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct ScheduleItem {
    /// Identifies this entry. The same song can be in a schedule twice, as
    /// two items with different ids.
    pub id: Id,
    /// Private notes for the operator, e.g. "Pastor prays after verse 2".
    pub notes: String,
    /// What the item shows.
    pub content: ItemContent,
}

/// What a schedule item shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum ItemContent {
    /// A song, copied from the library when it was added.
    Song {
        /// The song. Its id links it back to the library.
        song: Song,
    },
    /// Free text typed by the operator.
    Text {
        /// Title shown in the schedule.
        title: String,
        /// The text; blank lines separate slides.
        text: String,
    },
}

impl ScheduleItem {
    /// Creates an item with a fresh id and no notes.
    pub fn new(content: ItemContent) -> Self {
        Self {
            id: Id::new(),
            notes: String::new(),
            content,
        }
    }

    /// The title shown in the schedule list.
    pub fn title(&self) -> &str {
        match &self.content {
            ItemContent::Song { song } => &song.title,
            ItemContent::Text { title, .. } => title,
        }
    }

    /// Builds the slides for this item.
    pub fn presentation(&self, theme: &Theme) -> Presentation {
        match &self.content {
            ItemContent::Song { song } => {
                Presentation::from_song(song, theme.clone(), SlideOptions::default())
            }
            ItemContent::Text { title, text } => {
                Presentation::from_text(title.clone(), text, theme.clone())
            }
        }
    }
}

impl Schedule {
    /// The position of the item with `id`, if it is in the schedule.
    pub fn position(&self, id: Id) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    /// The item with `id`, if it is in the schedule.
    pub fn get(&self, id: Id) -> Option<&ScheduleItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// The item right after the one with `id`.
    pub fn after(&self, id: Id) -> Option<&ScheduleItem> {
        self.items.get(self.position(id)? + 1)
    }

    /// The item right before the one with `id`.
    pub fn before(&self, id: Id) -> Option<&ScheduleItem> {
        let position = self.position(id)?;
        self.items.get(position.checked_sub(1)?)
    }

    /// Inserts `item` at `index`, or at the end if `index` is `None` or past
    /// the end.
    pub fn insert(&mut self, item: ScheduleItem, index: Option<usize>) {
        let index = index.unwrap_or(self.items.len()).min(self.items.len());
        self.items.insert(index, item);
    }

    /// Moves the item with `id` so that it ends up at position `to`
    /// (clamped to the list). Returns `false` if there is no such item.
    pub fn move_item(&mut self, id: Id, to: usize) -> bool {
        let Some(from) = self.position(id) else {
            return false;
        };
        let item = self.items.remove(from);
        let to = to.min(self.items.len());
        self.items.insert(to, item);
        true
    }

    /// Removes the item with `id`. Returns `false` if there is no such item.
    pub fn remove(&mut self, id: Id) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        self.items.len() != before
    }

    /// Replaces every copy of `song` (matched by id) with the given version.
    /// Returns the ids of the items that changed.
    pub fn refresh_song(&mut self, song: &Song) -> Vec<Id> {
        let mut changed = Vec::new();
        for item in &mut self.items {
            if let ItemContent::Song { song: copy } = &mut item.content
                && copy.id == song.id
                && copy != song
            {
                *copy = song.clone();
                changed.push(item.id);
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_item(title: &str) -> ScheduleItem {
        ScheduleItem::new(ItemContent::Text {
            title: title.into(),
            text: "Some text".into(),
        })
    }

    fn titles(schedule: &Schedule) -> Vec<&str> {
        schedule.items.iter().map(ScheduleItem::title).collect()
    }

    fn schedule(names: &[&str]) -> Schedule {
        Schedule {
            items: names.iter().map(|name| text_item(name)).collect(),
        }
    }

    #[test]
    fn insert_appends_or_places_at_an_index() {
        let mut s = schedule(&["A", "C"]);
        s.insert(text_item("B"), Some(1));
        s.insert(text_item("D"), None);
        s.insert(text_item("E"), Some(99));
        assert_eq!(titles(&s), ["A", "B", "C", "D", "E"]);
    }

    #[test]
    fn move_item_puts_it_at_the_final_position() {
        let mut s = schedule(&["A", "B", "C", "D"]);
        let a = s.items[0].id;
        let d = s.items[3].id;

        assert!(s.move_item(a, 2));
        assert_eq!(titles(&s), ["B", "C", "A", "D"]);

        assert!(s.move_item(d, 0));
        assert_eq!(titles(&s), ["D", "B", "C", "A"]);

        assert!(!s.move_item(Id::new(), 0), "unknown ids are reported");
    }

    #[test]
    fn neighbours_and_removal() {
        let mut s = schedule(&["A", "B", "C"]);
        let b = s.items[1].id;

        assert_eq!(s.after(b).map(ScheduleItem::title), Some("C"));
        assert_eq!(s.before(b).map(ScheduleItem::title), Some("A"));
        assert_eq!(s.before(s.items[0].id), None);

        assert!(s.remove(b));
        assert!(!s.remove(b));
        assert_eq!(titles(&s), ["A", "C"]);
    }

    #[test]
    fn refresh_song_updates_every_copy() {
        let mut song = Song::new("Old");
        let mut s = Schedule::default();
        s.insert(
            ScheduleItem::new(ItemContent::Song { song: song.clone() }),
            None,
        );
        s.insert(text_item("Text"), None);
        s.insert(
            ScheduleItem::new(ItemContent::Song { song: song.clone() }),
            None,
        );

        song.title = "New".into();
        assert_eq!(s.refresh_song(&song).len(), 2);
        assert_eq!(titles(&s), ["New", "Text", "New"]);

        assert!(
            s.refresh_song(&song).is_empty(),
            "no change the second time"
        );
    }
}
