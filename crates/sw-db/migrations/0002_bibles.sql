-- Bible translations and their verses.
--
-- Unlike songs, Bibles are deleted for real (with their verses): they are
-- large, they come from files the operator can import again, and licences
-- mean they are never synced between computers.

CREATE TABLE bibles (
    seq           INTEGER PRIMARY KEY,
    id            TEXT    NOT NULL UNIQUE,
    name          TEXT    NOT NULL,
    abbreviation  TEXT    NOT NULL,
    language      TEXT    NOT NULL DEFAULT '',
    copyright     TEXT    NOT NULL DEFAULT '',
    -- For Bibles that ship with SimpleWorship, a stable key ("kjv", "web")
    -- so they are imported only once. NULL for the operator's own imports.
    bundled_key   TEXT    UNIQUE,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

-- `seq` (an alias for the rowid) lets the full-text index point at rows;
-- the unique index is what verse lookups use.
CREATE TABLE verses (
    seq        INTEGER PRIMARY KEY,
    bible_seq  INTEGER NOT NULL REFERENCES bibles (seq) ON DELETE CASCADE,
    book       INTEGER NOT NULL,
    chapter    INTEGER NOT NULL,
    verse      INTEGER NOT NULL,
    text       TEXT    NOT NULL
);

CREATE UNIQUE INDEX verses_position ON verses (bible_seq, book, chapter, verse);

-- Word search over verse text, e.g. "love is patient".
CREATE VIRTUAL TABLE verses_fts USING fts5 (
    text,
    content = 'verses',
    content_rowid = 'seq',
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TRIGGER verses_after_insert AFTER INSERT ON verses BEGIN
    INSERT INTO verses_fts (rowid, text) VALUES (new.seq, new.text);
END;

CREATE TRIGGER verses_after_delete AFTER DELETE ON verses BEGIN
    INSERT INTO verses_fts (verses_fts, rowid, text) VALUES ('delete', old.seq, old.text);
END;

CREATE TRIGGER verses_after_update AFTER UPDATE ON verses BEGIN
    INSERT INTO verses_fts (verses_fts, rowid, text) VALUES ('delete', old.seq, old.text);
    INSERT INTO verses_fts (rowid, text) VALUES (new.seq, new.text);
END;
