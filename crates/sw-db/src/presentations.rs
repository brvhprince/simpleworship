//! Imported presentations (slide decks). Their pages are media items whose
//! `collection` is the deck's id.

use rusqlite::{Connection, params};
use sw_core::{Id, SlideDeck, time};

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

    /// Records a new, empty deck and returns its id. Pages are added as
    /// media items with this id as their collection.
    pub fn create(&self, name: &str) -> Result<Id, DbError> {
        let id = Id::new();
        self.conn
            .prepare_cached("INSERT INTO presentations (id, name, created_at) VALUES (?1, ?2, ?3)")?
            .execute(params![id.to_string(), name, time::now()])?;
        Ok(id)
    }

    /// Every deck with its pages, newest first.
    pub fn list(&self) -> Result<Vec<SlideDeck>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT id, name, created_at FROM presentations ORDER BY created_at DESC",
        )?;
        let rows =
            statement.query_map([], |row| Ok((read_id(row, 0)?, row.get(1)?, row.get(2)?)))?;
        let media = Media::new(self.conn);
        let mut decks = Vec::new();
        for row in rows {
            let (id, name, created_at) = row?;
            decks.push(SlideDeck {
                id,
                name,
                pages: media.pages(id)?,
                created_at,
            });
        }
        Ok(decks)
    }

    /// Deletes a deck and its page records. Returns the pages, so the
    /// caller can delete their files.
    pub fn delete(&self, id: Id) -> Result<Vec<sw_core::MediaItem>, DbError> {
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
}

#[cfg(test)]
mod tests {
    use crate::Database;
    use sw_core::{Id, MediaItem, MediaKind};

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
        }
    }

    #[test]
    fn decks_keep_their_pages_out_of_the_media_tab() {
        let db = Database::open_in_memory().unwrap();
        let deck = db.presentations().create("Sermon").unwrap();
        db.media().save(&page(deck, "Page 002")).unwrap();
        db.media().save(&page(deck, "Page 001")).unwrap();

        let decks = db.presentations().list().unwrap();
        let names: Vec<_> = decks[0].pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Page 001", "Page 002"], "pages in order");
        assert!(
            db.media().list().unwrap().is_empty(),
            "not in the media library"
        );

        assert_eq!(db.presentations().delete(deck).unwrap().len(), 2);
        assert!(db.presentations().list().unwrap().is_empty());
    }
}
