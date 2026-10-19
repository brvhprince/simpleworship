# SimpleWorship

Free, open-source worship presentation software for macOS, Windows and Linux.
It puts lyrics, scripture and media on the projector, and anyone familiar
with EasyWorship should feel at home.

> **Status: early development.** The output engine and the song library work
> today; schedules, scripture, themes and media come next. See the
> [roadmap](#roadmap).

## What works today

- **Operator window** in the familiar layout: schedule, preview, live, and the
  library along the bottom.
- **Output and stage display** on any monitor, or in a window when you only
  have one screen. Outputs never steal keyboard focus from the operator.
  Testing on a single screen? A full-screen output on the operator's own
  screen stays *behind* the operator window, and Esc closes it.
- **Live controls:** Black, Logo, Clear Text, Clear Background, and cut or fade
  transitions.
- **Song library** with instant full-text search (titles, authors, lyrics,
  accent-insensitive), an editor with a live slide preview, section tags and
  a custom arrangement order.
- **Quick text:** type anything and show it.

## Keyboard shortcuts

| Key                                  | Action                                    |
| ------------------------------------ | ----------------------------------------- |
| → ↓ Space PageDown                   | Next slide                                |
| ← ↑ PageUp                           | Previous slide                            |
| Enter                                | Send preview live                         |
| F6 / F7 / F8 / F9                    | Black / Logo / Clear Text / Clear Background |
| Esc (in an output window)            | Close that output                         |
| V, C, B, P, T, I, E (+ a number)     | Jump to Verse 2, Chorus, Bridge, …        |

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
| M3        | Schedules: build, save/open, run a service                          |        |
| M4        | Scripture: Bible import, reference entry ("jn 3:16-18"), versions  |        |
| M5        | Theme designer, image/video backgrounds, media library              |        |
| M6        | Importers: EasyWorship 6/7, OpenLyrics, ProPresenter, ChordPro, CCLI |        |
| M7        | Alerts, countdowns, presentations (PDF/PPTX), CCLI usage report     |        |
| M8        | Installers, auto-update, translations                               |        |

Later: remote control from a phone, NDI / stream output, multi-operator
sync, and AI-assisted features.

## License

SimpleWorship is free software, released under the
[GNU General Public License v3.0 or later](LICENSE).
