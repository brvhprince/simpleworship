# Contributing to SimpleWorship

Thank you for helping! A few guidelines keep the code base pleasant to work in.

## Code style

**Readable code comes first.** Someone new to the project should be able to
open any file and follow it.

- **Format everything.** `pnpm format` runs rustfmt and Biome. CI rejects
  unformatted code.
- **Document as you go.**
  - Every Rust module starts with a `//!` comment saying what it is for.
  - Every public Rust item has a `///` doc comment. `missing_docs` is a
    warning, and warnings fail CI.
  - Exported TypeScript functions get a `/** … */` comment.
  - Svelte components start with an `@component` comment describing what
    they show and how to use them.
- **Comment the *why*, not the *what*.** Explain a surprising decision, a
  platform quirk or a trade-off. Don't restate the code.
- **Small, well-named functions** beat clever ones.

## Architecture rules

- `sw-core` stays **pure**: no files, no network, no threads. That keeps it
  easy to test and reusable by every front end.
- **All state changes go through the engine** as a `Command`. UI code never
  holds the "real" state; it mirrors what the engine reports.
- Types shared with TypeScript derive `ts_rs::TS`. After changing one, run
  `pnpm gen:bindings` and commit the generated files.
- Database changes are new files in `crates/sw-db/migrations/`. Never edit a
  migration that has been released.

- **Interface text is translated.** Never write words the operator reads
  straight into a component: add them to
  `packages/i18n/src/messages/en.json` and use `t("key")`. Then run
  `pnpm i18n:check --sync`, which adds the new key (empty) to the other
  languages for translators. See
  [Translating SimpleWorship](packages/i18n/src/messages/README.md).
- Releases are described in [docs/releasing.md](docs/releasing.md).

## Before opening a pull request

```sh
pnpm format
pnpm lint
pnpm check
pnpm test
pnpm i18n:check
```

## License

By contributing, you agree that your contributions are licensed under the
GPL-3.0-or-later, like the rest of the project.
