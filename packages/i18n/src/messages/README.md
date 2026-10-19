# Translating SimpleWorship

Each file here holds the interface's words in one language:

- `en.json`: English, complete. Every other file has the same keys.
- `fr.json`: French, complete.
- `tw.json`: Twi. Fill in the empty `""` values.

## How to translate

Open `tw.json` in any text editor and replace each empty `""` with the
Twi words for the English line with the same key in `en.json`. Leave a
line empty if you are unsure: SimpleWorship shows the English there.

```json
"toolbar.black": "Tuntum",
```

Keep these as they are:

- **`{name}` and the like:** SimpleWorship fills these in (a song title, a
  number). Put them where they belong in the sentence.
- **Counted lines** look like `{ "one": "…", "other": "…" }`: `one` for a
  single item ("1 song"), `other` for any other number ("3 songs").
- **Keyboard keys** such as `F6` or `Esc`.

Then choose Twi in **Settings › Language** to see it. Changes show after
SimpleWorship is restarted (or straight away in a development build).

`pnpm i18n:check` lists missing and extra keys, checks that the
`{placeholders}` match English, and says how much of each language is
translated. `pnpm i18n:check --sync` adds new English keys to every file
(empty, ready to translate).

**Adding a language:** copy `tw.json` to `<code>.json` (e.g. `ee.json` for
Ewe), add it to `LANGUAGES` and `PLURAL_LOCALES` in `../i18n.svelte.ts` (in `packages/i18n`),
and translate.
