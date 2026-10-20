<!--
  @component
  Picks the screen for this window. Every SimpleWorship window loads the same
  page; the Rust side opens it with `?view=output`, `?view=stage` or `?view=stream`
  (see `src-tauri/src/windows.rs`). Without a view it is the operator window.
-->
<script lang="ts">
  import type { MediaFolderPaths, MediaRef } from "@simpleworship/protocol";
  import { setMediaUrls } from "@simpleworship/renderer";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { i18n } from "./lib/i18n/i18n.svelte.ts";
  import Operator from "./operator/Operator.svelte";
  import Output from "./output/Output.svelte";
  import Stage from "./stage/Stage.svelte";
  import StreamWindow from "./stream/StreamWindow.svelte";

  interface Props {
    /** Where media files and thumbnails live (fetched in `main.ts`). */
    folders: MediaFolderPaths;
  }

  let { folders }: Props = $props();

  const view = new URLSearchParams(window.location.search).get("view") ?? "operator";

  // Media files are served by Tauri's asset protocol, limited to these
  // folders (see `assetProtocol` in tauri.conf.json).
  setMediaUrls({
    file: (media: MediaRef) => convertFileSrc(`${folders.media}/${media.file}`),
    thumbnail: (media: MediaRef) => convertFileSrc(`${folders.thumbnails}/${media.id}.jpg`),
  });

  // A language chosen in Settings reaches every window (the stage display too).
  onMount(() => i18n.listen());
</script>

{#if view === "output"}
  <Output />
{:else if view === "stage"}
  <Stage />
{:else if view === "stream"}
  <StreamWindow />
{:else}
  <Operator />
{/if}
