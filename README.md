# SimpleWorship

Free, open-source worship presentation software for macOS, Windows and Linux.
It puts lyrics, scripture and media on the projector, and anyone familiar
with EasyWorship should feel at home.

> **Status: early development.** The output engine, songs, schedules,
> scripture, themes, media, presentations and alerts work today. See the
> [roadmap](#roadmap).

## What works today

- **Operator window** in the familiar layout: schedule, preview, live, and the
  library along the bottom.
- **Output and stage display** on any monitor, or in a window when you only
  have one screen. With a projector connected, the output opens on it
  automatically at startup. Outputs never steal keyboard focus from the
  operator.
  Testing on a single screen? A full-screen output on the operator's own
  screen stays *behind* the operator window, and Esc closes it.
- **Live controls:** Black, Logo, Clear Text, Clear Background, and cut or fade
  transitions.
- **Song library** with instant full-text search (titles, authors, lyrics,
  accent-insensitive), an editor with a live slide preview, section tags and
  a custom arrangement order.
- **Quick text:** type anything and show it.
- **Schedules:** build the order of service by dragging songs in, reorder by
  dragging, and add notes for the operator. Run the whole service with one
  key: Next on the last slide of an item continues with the next item, and
  the following item is always waiting in preview.
- **Schedule files (`.sws`)** that carry their songs with them, so a schedule
  made at home opens on the church computer. Songs that the library has are
  taken from the library, so later corrections show up.
- **Crash protection:** every change is autosaved, and after a crash the
  schedule is offered back on the next start.
- **Scripture:** type a reference the way you would say it (`jn 3:16-18`,
  `1 cor 13`, `ps 23`, `rom 8:28, 31-39`) or pick book, chapter and verses.
  Search the text by words. Show two translations side by side. Passages
  are split into readable slides with verse numbers and a reference caption.
- **Bibles:** the King James Version and the World English Bible are
  included (both public domain). Import others from OSIS, Zefania,
  OpenSong, USX or USFM files. See [Bible licensing](resources/bibles/README.md).
- **Themes:** design how slides look, with a live preview: fonts, colours,
  outline, shadow, and a text area you drag into place. Backgrounds can be
  a colour, a gradient, an image or a looping video. Pick a default theme
  for songs, scripture and text; give a song its own theme; or change just
  one schedule item's theme or background.
- **Media:** an image and video library. Videos can be backgrounds or
  schedule items, with play, pause and seek in the live pane; every screen
  stays in step. A video background keeps playing smoothly while the lyrics
  change. Set any image as the logo for the Logo button.
- **Portable schedules:** saved schedules carry their themes and media, so
  they look the same on the church computer.
- **Bring your library across:** import songs from EasyWorship 6/7, OpenLP,
  ProPresenter 6, OpenLyrics, ChordPro and CCLI SongSelect files, and
  Bibles from EasyWorship. On first launch SimpleWorship looks for these
  programs and offers to import. See
  [Moving from EasyWorship](docs/moving-from-easyworship.md).
- **Alerts:** a nursery or car-park message over the slides, scrolling or
  still, at the top or bottom. Saved templates take a number in one step
  (“Parent of child 42…”). It clears itself after a few passes or seconds,
  or when you press Esc.
- **Countdowns:** “Service starts in 4:59”, to a time of day or for a number
  of minutes. At zero it stays on 0:00, or moves on to the next item.
- **Stage display:** the speaker's timer (counting up or down, red when over
  time) and messages only the stage sees (“Wrap up”), plus clock, next slide
  and text size options.
- **Background music:** play sound files (MP3, M4A, WAV…) from the Media tab
  or as a schedule item, with skip, seek, volume, loop and a fade-out. Music
  plays on the operator computer and carries on while slides change.
- **Presentations:** a built-in editor for announcements and sermon points:
  text slides (with **bold**, *italic* and underline), pictures and videos,
  in any order. A presentation has its own theme, and any slide can use a
  different theme or background, text size, colour and alignment. Set it to
  move on by itself every few seconds, looping, for announcements before the
  service.
- **Import PDF slides;** each page becomes a slide you can reorder, remove,
  or put typed slides between. PowerPoint and Keynote files are converted
  automatically when [LibreOffice](https://www.libreoffice.org) is
  installed; otherwise save them as PDF first.
- **Slideshows in the schedule:** drag several pictures onto the schedule
  at once for one item you step through like a song (hold Alt / Option for
  one item each). Drop pictures or a presentation onto a picture, text or
  presentation item to add them as slides, and drag slides in the preview
  to reorder them for that service.
- **Scripture styling in the theme:** verse numbers (on or off, colour, size,
  raised or inline, bold), and the reference as its own box: drag it
  anywhere, or one click for a corner, an edge or straight after the text,
  with its own size, colour and font. Show it on every slide or the first
  only. The song footer can be placed the same way. Long passages still
  flow onto the next slide when they would overflow the text box.
- **CCLI reporting:** every song shown live is logged once per day. The Song
  Usage Report (File menu) lists them for any period and exports CSV. Put
  your CCLI licence number in Settings and it appears in every song footer.

## Keyboard shortcuts

| Key                                  | Action                                    |
| ------------------------------------ | ----------------------------------------- |
| → ↓ Space PageDown                   | Next slide (into the next item at the end) |
| ← ↑ PageUp                           | Previous slide (into the previous item at the start) |
| Enter                                | Send preview live                         |
| F6 / F7 / F8 / F9                    | Black / Logo / Clear Text / Clear Background |
| Esc (in an output window)            | Close that output                         |
| Enter, Enter (in the Scripture tab)  | Preview the passage, then send it live    |
| ↑ / ↓ and Delete (in the schedule)   | Preview the previous / next item, remove the item |
| ⌘/Ctrl + N, O, S, Shift+S            | New, Open, Save, Save As schedule         |
| V, C, B, P, T, I, E (+ a number)     | Jump to Verse 2, Chorus, Bridge, …        |
| A                                    | Open the Alert panel                      |
| Esc                                  | Clear the alert                           |
| ⌘/Ctrl + ,                           | Settings                                  |

## Video formats

SimpleWorship plays videos with the system's own video support, so it
needs no extra codecs on macOS and Windows. **MP4 files with H.264 video
and AAC sound play everywhere** and are the safest choice. Other formats
(HEVC, WebM, MOV) depend on the computer.

On **Linux**, video playback uses GStreamer. Install its codec packages,
for example on Ubuntu or Debian:

```sh
sudo apt install gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav
```

## Getting started (development)

You need:

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 22+ and [pnpm](https://pnpm.io) 10+
- The [Tauri system dependencies](https://tauri.app/start/prerequisites/) for
  your OS. On Linux that includes WebKitGTK, and the GStreamer plugins for
  video playback.

```sh
pnpm install
pnpm dev        # runs the desktop app with hot reload
```

Other useful commands:

```sh
pnpm test           # Rust and TypeScript tests
pnpm check          # TypeScript / Svelte type checks
pnpm lint           # Biome + Clippy
pnpm format         # Biome + rustfmt
pnpm gen:bindings   # regenerate TypeScript types after changing Rust types
pnpm build          # build installers for your platform
```

## How it is built

SimpleWorship is a [Tauri 2](https://tauri.app) app: a Rust core with a
[Svelte 5](https://svelte.dev) user interface.

```
crates/
  sw-core/      Domain model and the presentation engine (pure Rust, no I/O)
  sw-db/        SQLite library storage with full-text search
  sw-files/     SimpleWorship's own file formats (.sws schedules)
  sw-bible/     Bible books, reference parsing, and Bible file importers
  sw-import/    Song importers for other worship software
apps/
  desktop/      The Tauri app: operator window, output and stage windows
packages/
  protocol/     TypeScript types generated from Rust, plus the Transport interface
  renderer/     Svelte components that draw slides (output, previews, thumbnails)
  ui/           Shared Svelte components and design tokens
```

The **Rust core owns all state.** Windows never change anything themselves.
They send a `Command` to the engine, and every window updates from the
`EngineEvent`s that come back. The same design will let a phone remote, a
second operator computer or an NDI output plug into the engine later,
without changes to the core.

Slides are laid out on a fixed 1920×1080 canvas and scaled to fit, so the
projector, the stage display and every thumbnail look the same.

## Roadmap

| Milestone | Scope                                                               | Status |
| --------- | ------------------------------------------------------------------- | ------ |
| M0        | Monorepo, tooling, CI                                               | ✅     |
| M1        | Output engine, monitors, live controls, transitions                 | ✅     |
| M2        | Song library, editor, search, preview → live                        | ✅     |
| M3        | Schedules: build, save/open, run a service                          | ✅     |
| M4        | Scripture: Bible import, reference entry ("jn 3:16-18"), versions  | ✅     |
| M5        | Theme designer, image/video backgrounds, media library, per-item overrides | ✅     |
| M6        | Importers: EasyWorship 6/7, OpenLP, OpenLyrics, ProPresenter, ChordPro, CCLI | ✅     |
| M7        | Alerts, countdowns, background audio, presentations (PDF/PPTX), CCLI report | ✅     |
| M8        | Presentation editor, slideshows, scripture styling (verse numbers, reference box) |        |
| M9        | Installers, auto-update, translations                               |        |

Later: remote control from a phone, NDI / stream output, multi-operator
sync, and AI-assisted features.

### Backlog

Planned, in order, after everything above:

1. Import the Bible Society of Ghana *Nkwa Asɛm* (NA-TWI) text files: one
   inconsistent text file per book, so it needs a careful reader that checks
   each chapter's verses and reports anything doubtful. (Twi already works
   today through EasyWorship `.ewb` import.)

## License

SimpleWorship is free software, released under the
[GNU General Public License v3.0 or later](LICENSE).
