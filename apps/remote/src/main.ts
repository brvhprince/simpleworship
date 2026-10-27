/**
 * Entry point of the phone remote. SimpleWorship serves this page to
 * phones and tablets on the church's network (see `crates/sw-server`).
 */

import "@simpleworship/ui/tokens.css";
import "@simpleworship/renderer/fonts.css";
import "./remote.css";
import { mount } from "svelte";
import App from "./App.svelte";

// Light or dark, as the phone is set (the tokens are dark unless told otherwise).
const light = window.matchMedia("(prefers-color-scheme: light)");
const followPhone = () => {
  document.documentElement.dataset.theme = light.matches ? "light" : "dark";
};
followPhone();
light.addEventListener("change", followPhone);

const target = document.getElementById("app");
if (!target) {
  throw new Error("index.html is missing the #app element");
}

export default mount(App, { target });
