//! The media library's records. The files themselves are managed by the
//! app, in its media folder.

use rusqlite::{Connection, OptionalExtension, Row, params};
use sw_core::{Id, MediaItem, MediaKind};

use crate::DbError;
use crate::songs::{read_id, read_optional_id};

/// Access to media records. Get one from [`Database::media`](crate::Database::media).
pub struct Media<'db> {
    conn: &'db Connection,
}

const COLUMNS: &str =
    "id, kind, name, file, width, height, duration_ms, size_bytes, created_at, collection";

impl<'db> Media<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// Every media item in the library, newest first. Presentation pages
    /// are left out (see [`Media::pages`]).
    pub fn list(&self) -> Result<Vec<MediaItem>, DbError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM media WHERE collection IS NULL
             ORDER BY created_at DESC, seq DESC"
        );
        let mut statement = self.conn.prepare_cached(&sql)?;
        let rows = statement.query_map([], read_media)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The pages of a presentation, in order.
    pub fn pages(&self, collection: Id) -> Result<Vec<MediaItem>, DbError> {
        let sql = format!("SELECT {COLUMNS} FROM media WHERE collection = ?1 ORDER BY name, seq");
        let mut statement = self.conn.prepare_cached(&sql)?;
        let rows = statement.query_map([collection.to_string()], read_media)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// One media item, if it exists.
    pub fn get(&self, id: Id) -> Result<Option<MediaItem>, DbError> {
        let sql = format!("SELECT {COLUMNS} FROM media WHERE id = ?1");
        let mut statement = self.conn.prepare_cached(&sql)?;
        Ok(statement
            .query_row([id.to_string()], read_media)
            .optional()?)
    }

    /// Stores a new item, or replaces the record of an existing one.
    pub fn save(&self, item: &MediaItem) -> Result<(), DbError> {
        self.conn
            .prepare_cached(
                "INSERT INTO media (id, kind, name, file, width, height, duration_ms,
                                    size_bytes, created_at, collection)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (id) DO UPDATE SET
                     name = excluded.name,
                     width = excluded.width,
                     height = excluded.height,
                     duration_ms = excluded.duration_ms",
            )?
            .execute(params![
                item.id.to_string(),
                kind_name(item.kind),
                item.name,
                item.file,
                item.width,
                item.height,
                item.duration_ms.map(to_sql_integer),
                to_sql_integer(item.size_bytes),
                item.created_at,
                item.collection.map(|id| id.to_string()),
            ])?;
        Ok(())
    }

    /// Removes an item's record and returns it, so the caller can delete
    /// the file. Returns `None` if there was no such item.
    pub fn delete(&self, id: Id) -> Result<Option<MediaItem>, DbError> {
        let item = self.get(id)?;
        if item.is_some() {
            self.conn
                .prepare_cached("DELETE FROM media WHERE id = ?1")?
                .execute([id.to_string()])?;
        }
        Ok(item)
    }
}

/// SQLite integers are signed 64-bit. Sizes and durations never come close
/// to the limit, but convert carefully rather than wrap around.
fn to_sql_integer(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn from_sql_integer(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn kind_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Video => "video",
        MediaKind::Audio => "audio",
    }
}

fn read_media(row: &Row<'_>) -> rusqlite::Result<MediaItem> {
    let kind: String = row.get(1)?;
    Ok(MediaItem {
        id: read_id(row, 0)?,
        kind: match kind.as_str() {
            "video" => MediaKind::Video,
            "audio" => MediaKind::Audio,
            _ => MediaKind::Image,
        },
        name: row.get(2)?,
        file: row.get(3)?,
        width: row.get(4)?,
        height: row.get(5)?,
        duration_ms: row.get::<_, Option<i64>>(6)?.map(from_sql_integer),
        size_bytes: from_sql_integer(row.get(7)?),
        created_at: row.get(8)?,
        collection: read_optional_id(row, 9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    fn item(name: &str, created_at: i64) -> MediaItem {
        MediaItem {
            id: Id::new(),
            kind: MediaKind::Video,
            name: name.into(),
            file: format!("{name}.mp4"),
            width: Some(1920),
            height: Some(1080),
            duration_ms: None,
            size_bytes: 123,
            created_at,
            collection: None,
        }
    }

    #[test]
    fn save_list_update_delete() {
        let db = Database::open_in_memory().unwrap();
        let media = db.media();
        let older = item("older", 1);
        let mut newer = item("newer", 2);
        media.save(&older).unwrap();
        media.save(&newer).unwrap();

        let names: Vec<_> = media.list().unwrap().into_iter().map(|m| m.name).collect();
        assert_eq!(names, ["newer", "older"]);

        // Details learned later (a video's length) are filled in.
        newer.duration_ms = Some(42_000);
        media.save(&newer).unwrap();
        assert_eq!(
            media.get(newer.id).unwrap().unwrap().duration_ms,
            Some(42_000)
        );

        assert_eq!(media.delete(older.id).unwrap(), Some(older.clone()));
        assert_eq!(media.delete(older.id).unwrap(), None);
    }
}
