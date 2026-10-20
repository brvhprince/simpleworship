<!--
  @component
  The stream look, filling the browser source. Set the browser source to
  1920×1080. Nothing else is ever drawn: no messages, no pointer, so a
  dropped connection never shows up on the livestream (the last picture
  stays until it reconnects).
-->
<script lang="ts">
  import type { MediaRef } from "@simpleworship/protocol";
  import { ScaledCanvas, StreamView, setMediaUrls } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { StreamConnection } from "./connection.svelte.ts";

  const key = new URLSearchParams(location.search).get("key") ?? "";
  const connection = new StreamConnection(key);

  setMediaUrls({
    file: (media: MediaRef) => `/media/${encodeURIComponent(media.file)}?token=${key}`,
    thumbnail: (media: MediaRef) => `/thumbnails/${media.id}.jpg?token=${key}`,
  });

  onMount(() => {
    connection.start();
    return () => connection.stop();
  });
</script>

{#if connection.output}
  <ScaledCanvas fit="contain" background={connection.output.stream.backdrop}>
    <StreamView output={connection.output} />
  </ScaledCanvas>
{/if}

<style>
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
    /* See-through, so OBS shows the camera behind the words. */
    background: transparent;
    cursor: none;
  }

  :global(#stream) {
    height: 100%;
  }
</style>
