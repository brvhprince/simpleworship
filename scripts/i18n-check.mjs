#!/usr/bin/env node
/**
 * Checks the interface translations against English
 * (packages/i18n/src/messages):
 *
 * - lists keys missing from a language, and keys English no longer has;
 * - checks that `{placeholders}` match English;
 * - says how much of each language is translated.
 *
 *   pnpm i18n:check          report only (fails on wrong placeholders)
 *   pnpm i18n:check --sync   also add missing keys (empty, for translators)
 *                            and remove keys English no longer has
 */

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const folder = join(dirname(fileURLToPath(import.meta.url)), "../packages/i18n/src/messages");
const sync = process.argv.includes("--sync");
const read = (file) => JSON.parse(readFileSync(join(folder, file), "utf8"));
const english = read("en.json");
const keys = Object.keys(english);

const texts = (message) => (typeof message === "string" ? [message] : Object.values(message));
const placeholders = (message) =>
  [...new Set(texts(message).flatMap((text) => text.match(/\{\w+\}/g) ?? []))].sort().join(",");
const done = (message) =>
  message !== undefined && texts(message).some((text) => text.trim() !== "");
const empty = (message) =>
  typeof message === "string"
    ? ""
    : Object.fromEntries(Object.keys(message).map((form) => [form, ""]));

let failed = false;
for (const file of readdirSync(folder).filter(
  (name) => name.endsWith(".json") && name !== "en.json",
)) {
  const language = read(file);
  const missing = keys.filter((key) => !(key in language));
  const extra = Object.keys(language).filter((key) => !(key in english));
  const translated = keys.filter((key) => done(language[key]));
  const wrong = translated.filter(
    (key) =>
      placeholders(language[key]) !== placeholders(english[key]) &&
      placeholders(language[key]) !== "",
  );
  const percent = Math.floor((translated.length / keys.length) * 100);
  console.log(`${file}: ${percent}% translated (${translated.length} of ${keys.length})`);
  if (missing.length) console.log(`  missing: ${missing.join(", ")}`);
  if (extra.length) console.log(`  no longer used: ${extra.join(", ")}`);
  for (const key of wrong) {
    console.log(
      `  ${key}: placeholders ${placeholders(language[key])} should be ${placeholders(english[key])}`,
    );
    failed = true;
  }
  if (sync && (missing.length || extra.length)) {
    const synced = Object.fromEntries(
      keys.map((key) => [key, key in language ? language[key] : empty(english[key])]),
    );
    writeFileSync(join(folder, file), `${JSON.stringify(synced, null, 2)}\n`);
    console.log(`  updated ${file}`);
  }
}
process.exit(failed ? 1 : 0);
