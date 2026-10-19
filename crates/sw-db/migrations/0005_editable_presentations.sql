-- Presentations become editable decks: text slides, pictures, themes.
-- The whole deck is kept as JSON (see `SlideDeck` in sw-core). Decks
-- imported from PDF before this have no JSON yet; they are read from their
-- pages until they are first saved.
ALTER TABLE presentations ADD COLUMN deck TEXT;
ALTER TABLE presentations ADD COLUMN updated_at INTEGER;
