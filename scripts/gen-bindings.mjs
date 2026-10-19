#!/usr/bin/env node
/**
 * Regenerates the TypeScript types in packages/protocol/src/generated from
 * the Rust types (via ts-rs), then formats them with Biome.
 *
 * ts-rs writes the files while `cargo test` runs its `export_bindings_*`
 * tests. A plain `cargo test` writes them to a git-ignored scratch folder
 * (see .cargo/config.toml); only this script points it at the real folder,
 * so running the tests never leaves the committed bindings changed.
 *
 * Usage: pnpm gen:bindings
 */

import { spawnSync } from "node:child_process";
import { copyFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "packages", "protocol", "src", "generated");

function run(command, args, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    env: { ...process.env, ...env },
    // pnpm/npx are .cmd files on Windows.
    shell: process.platform === "win32",
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

// Each crate's export also writes the types it *uses* from other crates,
// so a shared file (bindings.ts) would end up with only some of its types,
// depending on which crate ran last. So each crate exports into a folder of
// its own, and each file is taken from the crate that owns it.
const owners = {
  "bindings.ts": "sw-core",
  "bible.ts": "sw-bible",
  "import.ts": "sw-import",
  "remote.ts": "sw-server",
  "desktop.ts": "simpleworship-desktop",
};
const scratch = mkdtempSync(join(tmpdir(), "sw-bindings-"));
for (const crate of new Set(Object.values(owners))) {
  run("cargo", ["test", "-p", crate, "export_bindings"], {
    TS_RS_EXPORT_DIR: join(scratch, crate),
  });
}
for (const [file, crate] of Object.entries(owners)) {
  copyFileSync(join(scratch, crate, file), join(output, file));
}
rmSync(scratch, { recursive: true, force: true });
run("pnpm", ["exec", "biome", "check", "--write", output]);
