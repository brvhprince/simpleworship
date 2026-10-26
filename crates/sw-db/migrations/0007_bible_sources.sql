-- Where a Bible came from, when SimpleWorship downloaded it for the
-- operator: "ebible:<translation id>" for eBible.org's catalogue (see the
-- desktop app's `bible_catalog.rs`). NULL for Bibles that were bundled or
-- imported from a file.
ALTER TABLE bibles ADD COLUMN source TEXT;
