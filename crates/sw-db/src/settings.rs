//! Small application settings, stored as JSON values under string keys.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::DbError;

/// Access to settings. Get one from [`Database::settings`](crate::Database::settings).
pub struct Settings<'db> {
    conn: &'db Connection,
}

impl<'db> Settings<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// The value stored under `key`, or `None` if it was never set.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, DbError> {
        let json: Option<String> = self
            .conn
            .prepare_cached("SELECT value FROM settings WHERE key = ?1")?
            .query_row([key], |row| row.get(0))
            .optional()?;
        Ok(json.map(|json| serde_json::from_str(&json)).transpose()?)
    }

    /// Stores `value` under `key`, replacing any previous value.
    pub fn set<T: Serialize + ?Sized>(&self, key: &str, value: &T) -> Result<(), DbError> {
        let json = serde_json::to_string(value)?;
        self.conn
            .prepare_cached(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            )?
            .execute(params![key, json])?;
        Ok(())
    }

    /// Removes the value stored under `key`, if any.
    pub fn delete(&self, key: &str) -> Result<(), DbError> {
        self.conn
            .prepare_cached("DELETE FROM settings WHERE key = ?1")?
            .execute([key])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;

    #[test]
    fn stores_and_replaces_values() {
        let db = Database::open_in_memory().unwrap();
        let settings = db.settings();

        assert_eq!(settings.get::<String>("greeting").unwrap(), None);
        settings.set("greeting", "hello").unwrap();
        settings.set("greeting", "welcome").unwrap();
        assert_eq!(
            settings.get::<String>("greeting").unwrap().as_deref(),
            Some("welcome")
        );

        // `null` is a value too: "explicitly none".
        settings.set("logo", &None::<String>).unwrap();
        assert_eq!(settings.get::<Option<String>>("logo").unwrap(), Some(None));
    }
}
