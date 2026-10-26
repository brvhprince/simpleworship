-- How each picture or video fills the screen, chosen for the file itself
-- in the Media tab (see `MediaScaling` in sw-core): 'automatic', 'stretch',
-- 'maintain' or 'zoom'. Every file starts on automatic.
ALTER TABLE media ADD COLUMN scaling TEXT NOT NULL DEFAULT 'automatic';
