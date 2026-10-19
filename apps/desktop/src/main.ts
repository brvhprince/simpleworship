/**
 * Entry point for every SimpleWorship window. Which screen to show is
 * decided by `App.svelte` from the `?view=` query parameter.
 */

import "@simpleworship/ui/tokens.css";
import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("index.html is missing the #app element");
}

export default mount(App, { target });
