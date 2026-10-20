/**
 * Entry point of the stream page: the stream look (lower thirds) for OBS,
 * vMix or Streamlabs to show as a browser source, with a see-through
 * background. Served at `/stream?key=…` (see `crates/sw-server`).
 */

import "@simpleworship/renderer/fonts.css";
import { mount } from "svelte";
import Stream from "./stream/Stream.svelte";

const target = document.getElementById("stream");
if (!target) {
  throw new Error("stream.html is missing the #stream element");
}

export default mount(Stream, { target });
