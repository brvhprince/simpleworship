-- Audio, presentations, and the CCLI song usage log.

-- Allow audio in the media library, and group presentation pages.
-- SQLite cannot change a CHECK constraint, so the table is rebuilt; every
-- existing row is copied across unchanged.
CREATE TABLE media_new (
    seq          INTEGER PRIMARY KEY,
    id           TEXT    NOT NULL UNIQUE,
    kind         TEXT    NOT NULL CHECK (kind IN ('image', 'video', 'audio')),
    name         TEXT    NOT NULL,
    file         TEXT    NOT NULL,
    width        INTEGER,
    height       INTEGER,
    duration_ms  INTEGER,
    size_bytes   INTEGER NOT NULL,
    created_at   INTEGER NOT NULL,
    -- The presentation this is a page of, or NULL for library media.
    collection   TEXT
);

INSERT INTO media_new (seq, id, kind, name, file, width, height, duration_ms, size_bytes, created_at)
SELECT seq, id, kind, name, file, width, height, duration_ms, size_bytes, created_at FROM media;

DROP TABLE media;
ALTER TABLE media_new RENAME TO media;
CREATE INDEX media_collection ON media (collection);

-- Imported presentations. Their pages are media rows with `collection` set.
CREATE TABLE presentations (
    seq         INTEGER PRIMARY KEY,
    id          TEXT    NOT NULL UNIQUE,
    name        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

-- Every song shown live, at most once per song per day, for CCLI reports.
-- Title and details are copied so the report still works if the song is
-- later deleted or came from another computer's schedule.
CREATE TABLE song_usage (
    seq          INTEGER PRIMARY KEY,
    song_id      TEXT    NOT NULL,
    day          TEXT    NOT NULL,   -- local date, YYYY-MM-DD
    title        TEXT    NOT NULL,
    author       TEXT    NOT NULL DEFAULT '',
    copyright    TEXT    NOT NULL DEFAULT '',
    ccli_number  TEXT    NOT NULL DEFAULT '',
    shown_at     INTEGER NOT NULL,
    UNIQUE (song_id, day)
);

CREATE INDEX song_usage_day ON song_usage (day);
