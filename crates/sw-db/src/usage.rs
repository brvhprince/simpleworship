//! The CCLI song usage log: which songs were shown, on which days.

use rusqlite::{Connection, params};
use sw_core::usage::SongUsage;
use sw_core::{Id, Song, time};

use crate::DbError;

/// Access to the usage log. Get one from [`Database::usage`](crate::Database::usage).
pub struct Usage<'db> {
    conn: &'db Connection,
}

impl<'db> Usage<'db> {
    pub(crate) fn new(conn: &'db Connection) -> Self {
        Self { conn }
    }

    /// Records that a song was shown on `day` (local date, `YYYY-MM-DD`).
    /// A song counts once per day however often it is shown. `song` gives
    /// the details when the library has the song; otherwise only the title
    /// is known.
    pub fn record(
        &self,
        song_id: Id,
        title: &str,
        song: Option<&Song>,
        day: &str,
    ) -> Result<(), DbError> {
        let (author, copyright, ccli) = song.map_or(("", "", ""), |song| {
            (
                song.author.as_str(),
                song.copyright.as_str(),
                song.ccli_number.as_str(),
            )
        });
        self.conn
            .prepare_cached(
                "INSERT OR IGNORE INTO song_usage
                     (song_id, day, title, author, copyright, ccli_number, shown_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?
            .execute(params![
                song_id.to_string(),
                day,
                title,
                author,
                copyright,
                ccli,
                time::now()
            ])?;
        Ok(())
    }

    /// Every song used from `from` to `to` (inclusive, `YYYY-MM-DD`), most
    /// used first.
    pub fn report(&self, from: &str, to: &str) -> Result<Vec<SongUsage>, DbError> {
        let mut statement = self.conn.prepare_cached(
            "SELECT title, author, copyright, ccli_number, COUNT(*), MAX(day)
             FROM song_usage
             WHERE day BETWEEN ?1 AND ?2
             GROUP BY song_id
             ORDER BY COUNT(*) DESC, title",
        )?;
        let rows = statement.query_map(params![from, to], |row| {
            Ok(SongUsage {
                title: row.get(0)?,
                author: row.get(1)?,
                copyright: row.get(2)?,
                ccli_number: row.get(3)?,
                times: row.get(4)?,
                last_used: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;
    use sw_core::{Id, Song};

    #[test]
    fn counts_each_song_once_per_day() {
        let db = Database::open_in_memory().unwrap();
        let usage = db.usage();
        let mut grace = Song::new("Amazing Grace");
        grace.ccli_number = "22025".into();
        let other = Id::new();

        usage
            .record(grace.id, "Amazing Grace", Some(&grace), "2026-10-18")
            .unwrap();
        usage
            .record(grace.id, "Amazing Grace", Some(&grace), "2026-10-18")
            .unwrap();
        usage
            .record(grace.id, "Amazing Grace", Some(&grace), "2026-10-25")
            .unwrap();
        usage
            .record(other, "From another computer", None, "2026-10-25")
            .unwrap();

        let report = usage.report("2026-10-01", "2026-10-31").unwrap();
        assert_eq!(report.len(), 2);
        assert_eq!(
            (report[0].title.as_str(), report[0].times),
            ("Amazing Grace", 2)
        );
        assert_eq!(report[0].ccli_number, "22025");
        assert_eq!(report[0].last_used, "2026-10-25");

        assert_eq!(
            usage.report("2026-10-20", "2026-10-31").unwrap()[0].times,
            1
        );
    }
}
