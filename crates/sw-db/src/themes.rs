//! The theme library and the default theme for each kind of content.

use rusqlite::{Connection, Row, params};
use sw_core::themes::starter_library;
use sw_core::time;
use sw_core::{Id, ThemeDefaults, ThemeLibrary, ThemeRecord};

use crate::DbError;
use crate::settings::Settings;
use crate::songs::read_id;

/// Settings key under which the default themes are stored.
const DEFAULTS_KEY: &str = "themeDefaults";

/// Access to the themes. Get one from [`Database::themes`](crate::Database::themes).
pub struct Themes<'db> {
    conn: &'db Connection,
}

impl<'db> Themes<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// Adds the starter themes that have never been in this library, and
    /// sets the starter defaults if no defaults were chosen yet. A starter
    /// theme the operator deleted stays deleted.
    pub fn add_starters(&self) -> Result<(), DbError> {
        let starters = starter_library();
        for record in &starters.themes {
            let known = self
                .conn
                .prepare_cached("SELECT 1 FROM themes WHERE id = ?1")?
                .exists([record.id.to_string()])?;
            if !known {
                self.save(record)?;
            }
        }
        let settings = Settings::new(self.conn);
        if settings.get::<ThemeDefaults>(DEFAULTS_KEY)?.is_none() {
            settings.set(DEFAULTS_KEY, &starters.defaults)?;
        }
        Ok(())
    }

    /// Every theme (starter themes first, then by name) and the defaults.
    pub fn library(&self) -> Result<ThemeLibrary, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT id, theme, builtin FROM themes
             WHERE deleted_at IS NULL
             ORDER BY builtin DESC, created_at",
        )?;
        let rows = statement.query_map([], read_theme_row)?;
        let mut themes = Vec::new();
        for row in rows {
            let (id, json, builtin) = row?;
            themes.push(ThemeRecord {
                id,
                theme: serde_json::from_str(&json)?,
                builtin,
            });
        }

        let defaults = Settings::new(self.conn)
            .get(DEFAULTS_KEY)?
            .unwrap_or_default();
        Ok(ThemeLibrary { themes, defaults })
    }

    /// Creates or updates a theme (matched by id).
    pub fn save(&self, record: &ThemeRecord) -> Result<(), DbError> {
        let now = time::now();
        let json = serde_json::to_string(&record.theme)?;
        self.conn
            .prepare_cached(
                "INSERT INTO themes (id, theme, builtin, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT (id) DO UPDATE SET
                     theme = excluded.theme,
                     updated_at = excluded.updated_at,
                     deleted_at = NULL",
            )?
            .execute(params![record.id.to_string(), json, record.builtin, now])?;
        Ok(())
    }

    /// Marks a theme as deleted. Content that used it falls back to the
    /// default theme for its kind.
    pub fn delete(&self, id: Id) -> Result<bool, DbError> {
        let now = time::now();
        let changed = self
            .conn
            .prepare_cached(
                "UPDATE themes SET deleted_at = ?2, updated_at = ?2
                 WHERE id = ?1 AND deleted_at IS NULL",
            )?
            .execute(params![id.to_string(), now])?;
        Ok(changed > 0)
    }

    /// Chooses which theme each kind of content uses by default.
    pub fn set_defaults(&self, defaults: &ThemeDefaults) -> Result<(), DbError> {
        Settings::new(self.conn).set(DEFAULTS_KEY, defaults)
    }
}

fn read_theme_row(row: &Row<'_>) -> rusqlite::Result<(Id, String, bool)> {
    Ok((read_id(row, 0)?, row.get(1)?, row.get(2)?))
}

#[cfg(test)]
mod tests {
    use crate::Database;
    use sw_core::{Id, ThemeDefaults, ThemeRecord};

    #[test]
    fn a_new_library_has_the_starter_themes_and_defaults() {
        let db = Database::open_in_memory().unwrap();
        let library = db.themes().library().unwrap();

        let names: Vec<_> = library
            .themes
            .iter()
            .map(|t| t.theme.name.as_str())
            .collect();
        assert_eq!(names, ["Default", "Clean", "Scripture", "Lower Third"]);
        assert!(library.themes.iter().all(|t| t.builtin));
        assert_eq!(library.defaults.scripture, Some(library.themes[2].id));
    }

    #[test]
    fn save_edit_and_delete() {
        let db = Database::open_in_memory().unwrap();
        let themes = db.themes();

        let mut record = ThemeRecord {
            id: Id::new(),
            theme: sw_core::Theme::default(),
            builtin: false,
        };
        record.theme.name = "Christmas".into();
        themes.save(&record).unwrap();
        record.theme.text.color = "#ffd700".into();
        themes.save(&record).unwrap();

        let library = themes.library().unwrap();
        let stored = library.themes.iter().find(|t| t.id == record.id).unwrap();
        assert_eq!(stored.theme.text.color, "#ffd700");

        assert!(themes.delete(record.id).unwrap());
        assert_eq!(themes.library().unwrap().themes.len(), 4);
    }

    #[test]
    fn deleted_starter_themes_stay_deleted() {
        let db = Database::open_in_memory().unwrap();
        let clean = db.themes().library().unwrap().themes[1].id;
        db.themes().delete(clean).unwrap();

        db.themes().add_starters().unwrap();
        let ids: Vec<_> = db
            .themes()
            .library()
            .unwrap()
            .themes
            .iter()
            .map(|t| t.id)
            .collect();
        assert!(!ids.contains(&clean));
    }

    #[test]
    fn defaults_can_be_changed() {
        let db = Database::open_in_memory().unwrap();
        let clean = db.themes().library().unwrap().themes[1].id;
        let defaults = ThemeDefaults {
            song: Some(clean),
            ..db.themes().library().unwrap().defaults
        };
        db.themes().set_defaults(&defaults).unwrap();
        assert_eq!(db.themes().library().unwrap().defaults.song, Some(clean));
    }
}
