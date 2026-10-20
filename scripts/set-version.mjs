#!/usr/bin/env node
/**
 * Sets SimpleWorship's version everywhere it is written down: the Cargo
 * workspace (which the app and its installers use) and every package.json.
 *
 *   pnpm set-version 0.2.0
 *
 * Then add a section for it to CHANGELOG.md, commit, and tag `v0.2.0`
 * (see docs/releasing.md).
 */

import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2];

if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(version)) {
  console.error("Usage: pnpm set-version <major.minor.patch>, e.g. pnpm set-version 0.2.0");
  process.exit(1);
}

/** Replaces the first match of `pattern` in a file, failing loudly if there is none. */
function replace(file, pattern, replacement) {
  const path = join(root, file);
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) throw new Error(`No version found in ${file}`);
  writeFileSync(path, text.replace(pattern, replacement));
  console.log(`  ${file}`);
}

console.log(`Setting the version to ${version}:`);
// The workspace's own version, in [workspace.package].
replace("Cargo.toml", /(\[workspace\.package\][^[]*?\nversion = ")[^"]+(")/, `$1${version}$2`);
// Every app and package in the pnpm workspace, so new ones are never missed.
const packages = ["apps", "packages"].flatMap((folder) =>
  readdirSync(join(root, folder))
    .map((name) => `${folder}/${name}/package.json`)
    .filter((file) => existsSync(join(root, file))),
);
for (const file of packages) {
  replace(file, /("version":\s*")[^"]+(")/, `$1${version}$2`);
}
console.log("Done. Cargo.lock updates on the next build. Now update CHANGELOG.md.");
