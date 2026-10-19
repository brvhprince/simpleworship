-- Themes, media and settings.

-- Themes are stored as the JSON of `sw_core::Theme`. New theme settings get
-- defaults when an old theme is read, so the JSON never needs migrating.
-- Like songs, themes are only marked as deleted.
CREATE TABLE themes (
    seq         INTEGER PRIMARY KEY,
    id          TEXT    NOT NULL UNIQUE,
    theme       TEXT    NOT NULL,
    builtin     INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    deleted_at  INTEGER
);

-- Images and videos. The files live in the app's media folder under
-- `file`; deleting an item removes the row and (in the app) the file.
CREATE TABLE media (
    seq          INTEGER PRIMARY KEY,
    id           TEXT    NOT NULL UNIQUE,
    kind         TEXT    NOT NULL CHECK (kind IN ('image', 'video')),
    name         TEXT    NOT NULL,
    file         TEXT    NOT NULL,
    width        INTEGER,
    height       INTEGER,
    duration_ms  INTEGER,
    size_bytes   INTEGER NOT NULL,
    created_at   INTEGER NOT NULL
);

-- Small application settings (default themes, the logo, …) as JSON values.
CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
) WITHOUT ROWID;

-- A song's own theme, if it should not use the default song theme.
ALTER TABLE songs ADD COLUMN theme_id TEXT;
