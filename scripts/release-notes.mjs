#!/usr/bin/env node
/**
 * Prints the release notes for a version: its section of CHANGELOG.md.
 *
 *   node scripts/release-notes.mjs v0.2.0
 *
 * (Given something that is not a version, such as a branch name, it uses
 * the app's version from apps/desktop/package.json.)
 *
 * The release workflow uses this as the release's text, which is also what
 * installed copies show when they offer the update (the updater reads it
 * from the release's latest.json). Fails if CHANGELOG.md has no section for
 * the version, so a release can never go out without its notes.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
// A tag such as "v0.2.0", or (a manual run from a branch) the app's version.
const argument = (process.argv[2] ?? "").replace(/^v/, "");
const version = /^\d+\.\d+/.test(argument)
  ? argument
  : JSON.parse(readFileSync(join(root, "apps/desktop/package.json"), "utf8")).version;
if (!version) {
  console.error("Usage: node scripts/release-notes.mjs <version>, e.g. v0.2.0");
  process.exit(1);
}

const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8");
const lines = changelog.split("\n");
const start = lines.findIndex(
  (line) => line.startsWith(`## ${version} `) || line === `## ${version}`,
);
if (start < 0) {
  console.error(`CHANGELOG.md has no section for ${version}. Add "## ${version} (date)" first.`);
  process.exit(1);
}
const rest = lines.slice(start + 1);
const end = rest.findIndex((line) => line.startsWith("## "));
const section = (end < 0 ? rest : rest.slice(0, end)).join("\n").trim();
if (!section) {
  console.error(`The ${version} section of CHANGELOG.md is empty.`);
  process.exit(1);
}
console.log(section);
