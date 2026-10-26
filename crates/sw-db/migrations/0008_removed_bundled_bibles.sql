-- The bundled Bibles the operator deleted, by their key ("kjv", "web"…),
-- so the app does not install them again on its next start.
CREATE TABLE removed_bundled_bibles (
    key         TEXT    PRIMARY KEY,
    removed_at  INTEGER NOT NULL
);
