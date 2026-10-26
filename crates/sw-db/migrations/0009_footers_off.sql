-- The footer (a song's copyright and CCLI line, a translation's copyright)
-- is now off unless a theme turns it on. Existing themes had it on only
-- because that was the default, so it is turned off in them too; the
-- operator can turn it back on per theme in the theme designer.
UPDATE themes
SET theme = json_set(theme, '$.footer.visible', json('false'))
WHERE json_extract(theme, '$.footer.visible') = 1;
