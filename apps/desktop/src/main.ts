/**
 * Entry point for every SimpleWorship window. Which screen to show is
 * decided by `App.svelte` from the `?view=` query parameter.
 */

import "@simpleworship/ui/tokens.css";
import "@simpleworship/renderer/fonts.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { mediaFolders } from "./lib/api.ts";
import { appearance } from "./lib/appearance.svelte.ts";
import { link } from "./lib/link.svelte.ts";

// Light or dark before the first paint: the operator window follows the
// Appearance setting; the windows that show slides are always dark.
appearance.start(!new URLSearchParams(window.location.search).has("view"));

const target = document.getElementById("app");
if (!target) {
  throw new Error("index.html is missing the #app element");
}

// Media URLs are built from these folders (or, on a computer that joined
// another, from that computer's address), so fetch them before anything
// is drawn.
await link.load();
const folders = await mediaFolders();

export default mount(App, { target, props: { folders } });
