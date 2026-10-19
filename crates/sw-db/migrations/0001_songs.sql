-- Songs and their full-text search index.
--
-- `seq` is an INTEGER PRIMARY KEY, which makes it an alias for SQLite's rowid.
-- The FTS index refers to rows by rowid, and an *aliased* rowid is guaranteed
-- never to change (a plain hidden rowid may be renumbered by VACUUM). The
-- public identifier is the UUID in `id`.
--
-- Rows are never physically deleted: `deleted_at` marks them as removed, so a
-- future sync between computers can tell "deleted" apart from "never had it".

CREATE TABLE songs (
    seq          INTEGER PRIMARY KEY,
    id           TEXT    NOT NULL UNIQUE,
    title        TEXT    NOT NULL,
    author       TEXT    NOT NULL DEFAULT '',
    copyright    TEXT    NOT NULL DEFAULT '',
    ccli_number  TEXT    NOT NULL DEFAULT '',
    lyrics       TEXT    NOT NULL DEFAULT '',
    arrangement  TEXT    NOT NULL DEFAULT '',
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    deleted_at   INTEGER
);

CREATE INDEX songs_title ON songs (title COLLATE NOCASE) WHERE deleted_at IS NULL;

-- External-content FTS5 table: the text lives in `songs`, this only stores
-- the index. `remove_diacritics 2` lets "senor" find "señor".
CREATE VIRTUAL TABLE songs_fts USING fts5 (
    title,
    author,
    lyrics,
    content = 'songs',
    content_rowid = 'seq',
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Keep the index in sync with the table.
CREATE TRIGGER songs_after_insert AFTER INSERT ON songs BEGIN
    INSERT INTO songs_fts (rowid, title, author, lyrics)
    VALUES (new.seq, new.title, new.author, new.lyrics);
END;

CREATE TRIGGER songs_after_delete AFTER DELETE ON songs BEGIN
    INSERT INTO songs_fts (songs_fts, rowid, title, author, lyrics)
    VALUES ('delete', old.seq, old.title, old.author, old.lyrics);
END;

CREATE TRIGGER songs_after_update AFTER UPDATE ON songs BEGIN
    INSERT INTO songs_fts (songs_fts, rowid, title, author, lyrics)
    VALUES ('delete', old.seq, old.title, old.author, old.lyrics);
    INSERT INTO songs_fts (rowid, title, author, lyrics)
    VALUES (new.seq, new.title, new.author, new.lyrics);
END;
