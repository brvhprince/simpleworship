//! Presentations (slide decks): made in the editor, or imported from PDF.
//!
//! Each deck is stored whole, as JSON. Pages imported from a PDF are media
//! items whose `collection` is the deck's id; they stay out of the media
//! library and are deleted with the deck.

use rusqlite::{Connection, OptionalExtension, params};
use sw_core::{DeckSlide, Id, MediaItem, SlideDeck, time};

use crate::DbError;
use crate::media::Media;
use crate::songs::read_id;

/// Access to slide decks. Get one from [`Database::presentations`](crate::Database::presentations).
pub struct Presentations<'db> {
    conn: &'db Connection,
}

impl<'db> Presentations<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// Records a new, empty deck for a PDF import and returns its id. Pages
    /// are added as media items with this id as their collection; the deck
    /// is then [saved](Presentations::save) once they are all in.
    pub fn create(&self, name: &str) -> Result<Id, DbError> {
        let id = Id::new();
        self.conn
            .prepare_cached("INSERT INTO presentations (id, name, created_at) VALUES (?1, ?2, ?3)")?
            .execute(params![id.to_string(), name, time::now()])?;
        Ok(id)
    }

    /// Saves a deck, adding it if it is new. Sets its `updated_at`.
    pub fn save(&self, deck: &SlideDeck) -> Result<SlideDeck, DbError> {
        let mut deck = deck.clone();
        deck.updated_at = time::now();
        self.conn
            .prepare_cached(
                "INSERT INTO presentations (id, name, created_at, updated_at, deck)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (id) DO UPDATE SET
                     name = excluded.name,
                     updated_at = excluded.updated_at,
                     deck = excluded.deck",
            )?
            .execute(params![
                deck.id.to_string(),
                deck.name,
                deck.created_at,
                deck.updated_at,
                serde_json::to_string(&deck)?,
            ])?;
        Ok(deck)
    }

    /// Every deck, most recently changed first.
    pub fn list(&self) -> Result<Vec<SlideDeck>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT id, name, created_at, deck FROM presentations
             ORDER BY COALESCE(updated_at, created_at) DESC, seq DESC",
        )?;
        let rows = statement.query_map([], read_row)?;
        let rows: Vec<_> = rows.collect::<Result<_, _>>()?;
        rows.into_iter().map(|row| self.deck_from(row)).collect()
    }

    /// One deck, if it exists.
    pub fn get(&self, id: Id) -> Result<Option<SlideDeck>, DbError> {
        let row = self
            .conn
            .prepare_cached("SELECT id, name, created_at, deck FROM presentations WHERE id = ?1")?
            .query_row([id.to_string()], read_row)
            .optional()?;
        row.map(|row| self.deck_from(row)).transpose()
    }

    /// Deletes a deck and its imported pages' records. Returns the pages, so
    /// the caller can delete their files.
    pub fn delete(&self, id: Id) -> Result<Vec<MediaItem>, DbError> {
        let pages = Media::new(self.conn).pages(id)?;
        let transaction = self.conn.unchecked_transaction()?;
        transaction
            .prepare_cached("DELETE FROM media WHERE collection = ?1")?
            .execute([id.to_string()])?;
        transaction
            .prepare_cached("DELETE FROM presentations WHERE id = ?1")?
            .execute([id.to_string()])?;
        transaction.commit()?;
        Ok(pages)
    }

    /// The deck from its JSON, or, for a PDF imported before decks could be
    /// edited, one picture slide per imported page.
    fn deck_from(&self, row: Row) -> Result<SlideDeck, DbError> {
        let (id, name, created_at, json) = row;
        if let Some(json) = json {
            return Ok(serde_json::from_str(&json)?);
        }
        let pages = Media::new(self.conn).pages(id)?;
        Ok(SlideDeck {
            id,
            name,
            slides: pages.iter().filter_map(DeckSlide::picture).collect(),
            created_at,
            updated_at: created_at,
            ..SlideDeck::new("")
        })
    }
}

/// `(id, name, created_at, deck JSON)`.
type Row = (Id, String, i64, Option<String>);

fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok((read_id(row, 0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

#[cfg(test)]
mod tests {
    use crate::Database;
    use sw_core::{DeckSlide, Id, MediaItem, MediaKind, SlideDeck};

    fn page(collection: Id, name: &str) -> MediaItem {
        MediaItem {
            id: Id::new(),
            kind: MediaKind::Image,
            name: name.into(),
            file: format!("{name}.png"),
            width: Some(1920),
            height: Some(1080),
            duration_ms: None,
            size_bytes: 10,
            created_at: 0,
            collection: Some(collection),
            scaling: sw_core::MediaScaling::Automatic,
        }
    }

    #[test]
    fn imported_pages_become_picture_slides_until_saved() {
        let db = Database::open_in_memory().unwrap();
        let id = db.presentations().create("Sermon").unwrap();
        db.media().save(&page(id, "Page 002")).unwrap();
        db.media().save(&page(id, "Page 001")).unwrap();

        let deck = db.presentations().get(id).unwrap().unwrap();
        assert_eq!(deck.name, "Sermon");
        assert_eq!(deck.slides.len(), 2);
        assert!(
            db.media().list().unwrap().is_empty(),
            "pages stay out of the media tab"
        );

        // Add a typed slide between the pages and save.
        let mut edited = deck.clone();
        edited.slides.insert(1, DeckSlide::text("Questions?"));
        db.presentations().save(&edited).unwrap();
        let reread = db.presentations().get(id).unwrap().unwrap();
        assert_eq!(reread.slides[1].text, "Questions?");
        assert_eq!(db.presentations().list().unwrap().len(), 1);

        assert_eq!(db.presentations().delete(id).unwrap().len(), 2);
        assert!(db.presentations().list().unwrap().is_empty());
    }

    #[test]
    fn decks_made_in_the_editor_are_saved_whole() {
        let db = Database::open_in_memory().unwrap();
        let mut deck = SlideDeck::new("Notices");
        deck.slides = vec![DeckSlide::text("**Choir** at 4pm")];
        db.presentations().save(&deck).unwrap();

        let saved = db.presentations().get(deck.id).unwrap().unwrap();
        assert_eq!(saved.slides, deck.slides);
        assert!(saved.updated_at >= deck.created_at);
    }
}
