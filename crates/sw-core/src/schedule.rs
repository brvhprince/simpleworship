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

use sw_bible::Passage;

use crate::countdown::CountdownSlide;
use crate::deck::{DeckSlide, SlideDeck};
use crate::id::Id;
use crate::media::{CameraRef, MediaItem};
use crate::presentation::Presentation;
use crate::scripture::{self, ScriptureOptions, TranslationText};
use crate::slides::{self, SlideOptions};
use crate::song::Song;
use crate::themes::{ContentKind, ItemLook, ThemeLibrary};

/// The order of service.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "bindings.ts")]
pub struct Schedule {
    /// The items, in service order.
    pub items: Vec<ScheduleItem>,
}

/// One entry in a schedule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
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
    /// A theme or background for this item only.
    #[serde(default)]
    pub look: ItemLook,
}

/// What a schedule item shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
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
    /// A Bible passage in one or more translations, with a copy of the text.
    Scripture {
        /// The passage.
        passage: Passage,
        /// The passage written out, e.g. "John 3:16–18", for showing in
        /// the schedule without needing the book names.
        reference: String,
        /// The text in each translation, shown side by side.
        translations: Vec<TranslationText>,
        /// How the passage is split into slides.
        options: ScriptureOptions,
    },
    /// An image or video from the media library, shown on its own.
    Media {
        /// The media item (the file travels with a saved schedule).
        media: MediaItem,
    },
    /// A camera's live picture, filling the screen.
    Camera {
        /// Which camera.
        camera: CameraRef,
    },
    /// A countdown, e.g. to the start of the service.
    Countdown {
        /// Title in the schedule, e.g. "Countdown to 10:30".
        title: String,
        /// What it counts down to and what happens at zero.
        countdown: CountdownSlide,
    },
    /// A presentation, imported PDF or slideshow of pictures: a copy of
    /// the deck, so slides can be added or reordered for this service only.
    Deck {
        /// The deck. Its id links it back to the library (a slideshow made
        /// in the schedule is not in the library).
        deck: SlideDeck,
    },
    /// An imported presentation (PDF or PowerPoint): one picture per page.
    /// Only in schedules saved before decks could be edited; adding slides
    /// to it turns it into a [`ItemContent::Deck`].
    Slides {
        /// The presentation's name.
        name: String,
        /// Its pages, in order (the files travel with a saved schedule).
        pages: Vec<MediaItem>,
    },
    /// Background music. Sending it live plays it; the screen stays as it is.
    Playlist {
        /// The playlist's name.
        name: String,
        /// The tracks, in order (the files travel with a saved schedule).
        tracks: Vec<MediaItem>,
    },
    /// A sermon's points and passages, in the order of the sermon, as one
    /// item (made from the preacher's notes by the assistant), so they do
    /// not fill the schedule. Points are drawn with the text theme and
    /// passages with the scripture theme.
    Sermon {
        /// Title in the schedule, e.g. "Sermon: He sees you".
        title: String,
        /// The points and passages, in order.
        parts: Vec<SermonPart>,
    },
}

/// One part of a [`ItemContent::Sermon`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "bindings.ts")]
pub enum SermonPart {
    /// A point, shown as text.
    Point {
        /// Its heading, e.g. "1. In the valley", shown above its slides in
        /// the slide list (and on the slide when there is no text).
        title: String,
        /// The words on the slide; blank lines separate slides.
        text: String,
    },
    /// A Bible passage, with a copy of the text, as in
    /// [`ItemContent::Scripture`].
    Scripture {
        /// The passage.
        passage: Passage,
        /// The passage written out, e.g. "Psalm 23:4".
        reference: String,
        /// The text in each translation, shown side by side.
        translations: Vec<TranslationText>,
        /// How the passage is split into slides.
        options: ScriptureOptions,
    },
}

impl ScheduleItem {
    /// Creates an item with a fresh id and no notes.
    pub fn new(content: ItemContent) -> Self {
        Self {
            id: Id::new(),
            notes: String::new(),
            content,
            look: ItemLook::default(),
        }
    }

    /// The title shown in the schedule list.
    pub fn title(&self) -> String {
        match &self.content {
            ItemContent::Song { song } => song.title.clone(),
            ItemContent::Text { title, .. } => title.clone(),
            ItemContent::Scripture {
                passage,
                translations,
                ..
            } => scripture::title(passage, translations),
            ItemContent::Media { media } => media.name.clone(),
            ItemContent::Camera { camera } => camera.label.clone(),
            ItemContent::Countdown { title, .. } => title.clone(),
            ItemContent::Slides { name, .. } | ItemContent::Playlist { name, .. } => name.clone(),
            ItemContent::Deck { deck } => deck.name.clone(),
            ItemContent::Sermon { title, .. } => title.clone(),
        }
    }

    /// Builds the slides for this item, styled from `themes` (see
    /// [`crate::themes`] for how the theme is chosen).
    pub fn presentation(&self, themes: &ThemeLibrary) -> Presentation {
        let look = |kind, own_theme| themes.resolve(kind, own_theme, &self.look);
        match &self.content {
            ItemContent::Song { song } => Presentation::from_song(
                song,
                look(ContentKind::Song, song.theme_id),
                SlideOptions::default(),
            ),
            ItemContent::Text { title, text } => {
                Presentation::from_text(title.clone(), text, look(ContentKind::Text, None))
            }
            ItemContent::Scripture {
                passage,
                translations,
                options,
                ..
            } => Presentation::from_scripture(
                passage,
                translations,
                *options,
                look(ContentKind::Scripture, None),
            ),
            ItemContent::Media { media } => {
                Presentation::from_media(media, look(ContentKind::Text, None))
            }
            ItemContent::Camera { camera } => {
                Presentation::from_camera(camera, look(ContentKind::Text, None))
            }
            ItemContent::Countdown { title, countdown } => Presentation::from_countdown(
                title.clone(),
                countdown.clone(),
                look(ContentKind::Text, None),
            ),
            ItemContent::Slides { name, pages } => {
                Presentation::from_pages(name.clone(), pages, look(ContentKind::Text, None))
            }
            ItemContent::Playlist { name, tracks } => {
                Presentation::from_playlist(name.clone(), tracks, look(ContentKind::Text, None))
            }
            ItemContent::Deck { deck } => Presentation::from_deck(
                deck,
                look(ContentKind::Presentation, deck.theme_id),
                themes,
            ),
            ItemContent::Sermon { title, parts } => {
                Presentation::from_sermon(title.clone(), parts, |kind| look(kind, None))
            }
        }
    }

    /// Turns the item into a deck if it can hold more slides: pictures,
    /// videos, typed text and imported pages can; songs, scripture,
    /// countdowns and music cannot. Returns the deck to change, or `None`.
    pub fn as_deck(&mut self) -> Option<&mut SlideDeck> {
        let deck = match &self.content {
            ItemContent::Deck { .. } => None,
            ItemContent::Media { media } => Some(SlideDeck::slideshow(
                media.name.clone(),
                std::slice::from_ref(media),
            )),
            ItemContent::Slides { name, pages } => Some(SlideDeck::slideshow(name.clone(), pages)),
            ItemContent::Text { title, text } => Some(SlideDeck {
                slides: slides::slides_from_text(text, SlideOptions::default())
                    .into_iter()
                    .map(|slide| DeckSlide::text(slide.text))
                    .collect(),
                ..SlideDeck::new(title.clone())
            }),
            ItemContent::Song { .. }
            | ItemContent::Sermon { .. }
            | ItemContent::Scripture { .. }
            | ItemContent::Countdown { .. }
            | ItemContent::Camera { .. }
            | ItemContent::Playlist { .. } => return None,
        };
        if let Some(deck) = deck {
            self.content = ItemContent::Deck { deck };
        }
        match &mut self.content {
            ItemContent::Deck { deck } => Some(deck),
            _ => None,
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

    /// The item with `id`, for changing.
    pub fn get_mut(&mut self, id: Id) -> Option<&mut ScheduleItem> {
        self.items.iter_mut().find(|item| item.id == id)
    }

    /// Replaces every copy of `deck` (matched by id) with the given version.
    /// Returns the ids of the items that changed.
    pub fn refresh_deck(&mut self, deck: &SlideDeck) -> Vec<Id> {
        let mut changed = Vec::new();
        for item in &mut self.items {
            if let ItemContent::Deck { deck: copy } = &mut item.content
                && copy.id == deck.id
                && copy != deck
            {
                *copy = deck.clone();
                changed.push(item.id);
            }
        }
        changed
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

    fn titles(schedule: &Schedule) -> Vec<String> {
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

        assert_eq!(s.after(b).map(ScheduleItem::title).as_deref(), Some("C"));
        assert_eq!(s.before(b).map(ScheduleItem::title).as_deref(), Some("A"));
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
