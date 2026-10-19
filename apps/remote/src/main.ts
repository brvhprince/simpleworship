/**
 * Entry point of the phone remote. SimpleWorship serves this page to
 * phones and tablets on the church's network (see `crates/sw-server`).
 */

import "@simpleworship/ui/tokens.css";
import "@simpleworship/renderer/fonts.css";
import "./remote.css";
import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("index.html is missing the #app element");
}

export default mount(App, { target });
