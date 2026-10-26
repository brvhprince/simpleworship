<!--
  @component
  Picks the screen for this window. Every SimpleWorship window loads the same
  page; the Rust side opens it with `?view=output`, `?view=stage`, `?view=stream` or `?view=ndi`
  (see `src-tauri/src/windows.rs`). Without a view it is the operator window.
-->
<script lang="ts">
  import type { MediaFolderPaths, MediaRef } from "@simpleworship/protocol";
  import { setCameraUse, setFileFitLookup, setMediaUrls } from "@simpleworship/renderer";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { i18n } from "./lib/i18n/i18n.svelte.ts";
  import { link } from "./lib/link.svelte.ts";
  import { media } from "./lib/media.svelte.ts";
  import NdiPage from "./ndi/NdiPage.svelte";
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
  // folders (see `assetProtocol` in tauri.conf.json). A computer that
  // joined another shows that computer's, from its server.
  const host = link.status;
  setMediaUrls(
    host.joined
      ? {
          file: (media: MediaRef) =>
            `${host.address}/media/${encodeURIComponent(media.file)}?token=${host.token}`,
          thumbnail: (media: MediaRef) =>
            `${host.address}/thumbnails/${media.id}.jpg?token=${host.token}`,
        }
      : {
          file: (media: MediaRef) => convertFileSrc(`${folders.media}/${media.file}`),
          thumbnail: (media: MediaRef) => convertFileSrc(`${folders.thumbnails}/${media.id}.jpg`),
        },
  );

  // Cameras: live on the projector and the operator's panes (which also
  // say when one is missing); never opened by NDI's hidden windows, whose
  // pictures could not keep up with live video.
  // A joined computer's cameras are not the service's: the main
  // computer's cameras show there only.
  // Each file's own Scaling, for backgrounds drawn without the engine (the
  // theme designer and slide editor). Only the operator window loads the
  // library; the screens get the same fits from the engine.
  setFileFitLookup((mediaId) => media.fileFits.get(mediaId) ?? null);

  setCameraUse({
    live: view !== "ndi" && view !== "stage" && !host.joined,
    placeholder: view === "operator",
    explain: view === "operator",
  });

  // A language chosen in Settings reaches every window (the stage display
  // too); joining or leaving another computer reloads every window.
  onMount(() => {
    const stopLanguage = i18n.listen();
    const linked = link.connect();
    return () => {
      stopLanguage();
      void linked.then((stop) => stop());
    };
  });
</script>

{#if view === "output"}
  <Output />
{:else if view === "stage"}
  <Stage />
{:else if view === "stream"}
  <StreamWindow />
{:else if view === "ndi"}
  <NdiPage />
{:else}
  <Operator />
{/if}
