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

run("cargo", ["test", "--workspace", "export_bindings"], { TS_RS_EXPORT_DIR: output });
run("pnpm", ["exec", "biome", "check", "--write", output]);
