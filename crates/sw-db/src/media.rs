//! The media library's records. The files themselves are managed by the
//! app, in its media folder.

use rusqlite::{Connection, OptionalExtension, Row, params};
use sw_core::{FileFits, Id, MediaItem, MediaKind, MediaScaling};

use crate::DbError;
use crate::songs::{read_id, read_optional_id};

/// Access to media records. Get one from [`Database::media`](crate::Database::media).
pub struct Media<'db> {
    conn: &'db Connection,
}

const COLUMNS: &str = "id, kind, name, file, width, height, duration_ms, size_bytes, created_at, \
                       collection, scaling";

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

    /// Stores a new item, or updates an existing one's name and details.
    /// An existing item keeps its scaling: that changes only through
    /// [`Media::set_scaling`], so a schedule from another computer does not
    /// overwrite the choice made here.
    pub fn save(&self, item: &MediaItem) -> Result<(), DbError> {
        self.conn
            .prepare_cached(
                "INSERT INTO media (id, kind, name, file, width, height, duration_ms,
                                    size_bytes, created_at, collection, scaling)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
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
                scaling_name(item.scaling),
            ])?;
        Ok(())
    }

    /// Sets how a file fills the screen. Returns whether it exists.
    pub fn set_scaling(&self, id: Id, scaling: MediaScaling) -> Result<bool, DbError> {
        let changed = self
            .conn
            .prepare_cached("UPDATE media SET scaling = ?2 WHERE id = ?1")?
            .execute(params![id.to_string(), scaling_name(scaling)])?;
        Ok(changed > 0)
    }

    /// The files not on automatic, and the fit each asks for: what the
    /// engine applies to backgrounds (see `Command::SetFileFits`).
    pub fn file_fits(&self) -> Result<FileFits, DbError> {
        let mut statement = self
            .conn
            .prepare_cached("SELECT id, scaling FROM media WHERE scaling <> 'automatic'")?;
        let rows = statement.query_map([], |row| {
            let scaling: String = row.get(1)?;
            Ok((read_id(row, 0)?, parse_scaling(&scaling)))
        })?;
        let mut fits = FileFits::new();
        for row in rows {
            let (id, scaling) = row?;
            if let Some(fit) = scaling.fit() {
                fits.insert(id, fit);
            }
        }
        Ok(fits)
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

fn scaling_name(scaling: MediaScaling) -> &'static str {
    match scaling {
        MediaScaling::Automatic => "automatic",
        MediaScaling::Stretch => "stretch",
        MediaScaling::Maintain => "maintain",
        MediaScaling::Zoom => "zoom",
    }
}

/// The scaling stored as `name`; anything unknown is automatic.
fn parse_scaling(name: &str) -> MediaScaling {
    match name {
        "stretch" => MediaScaling::Stretch,
        "maintain" => MediaScaling::Maintain,
        "zoom" => MediaScaling::Zoom,
        _ => MediaScaling::Automatic,
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
        scaling: parse_scaling(&row.get::<_, String>(10)?),
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
            scaling: MediaScaling::Automatic,
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

    #[test]
    fn scaling_is_kept_and_listed_for_the_engine() {
        let db = Database::open_in_memory().unwrap();
        let media = db.media();
        let mut photo = item("photo", 1);
        let other = item("other", 2);
        media.save(&photo).unwrap();
        media.save(&other).unwrap();
        assert!(
            media.file_fits().unwrap().is_empty(),
            "all automatic at first"
        );

        assert!(media.set_scaling(photo.id, MediaScaling::Zoom).unwrap());
        assert_eq!(
            media.get(photo.id).unwrap().unwrap().scaling,
            MediaScaling::Zoom
        );
        assert_eq!(
            media.file_fits().unwrap(),
            FileFits::from([(photo.id, sw_core::MediaFit::Cover)])
        );

        // Renaming (or a schedule from elsewhere) does not undo the choice.
        photo.name = "renamed".into();
        media.save(&photo).unwrap();
        assert_eq!(
            media.get(photo.id).unwrap().unwrap().scaling,
            MediaScaling::Zoom
        );

        assert!(!media.set_scaling(Id::new(), MediaScaling::Zoom).unwrap());
    }
}
