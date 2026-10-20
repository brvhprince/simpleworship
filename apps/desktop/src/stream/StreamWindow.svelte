<!--
  @component
  The stream output window: the stream look (lower thirds), full screen,
  for a capture card or a hardware switcher. A window cannot be see-through
  on a screen, so a transparent backdrop shows as black here; choose green
  in Settings › Stream for a switcher that keys out a colour.
-->
<script lang="ts">
  import { ScaledCanvas, StreamView } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { closeOnEscape } from "../lib/output-window.ts";

  onMount(() => {
    const connection = engine.connect();
    return () => void connection.then((disconnect) => disconnect());
  });
</script>

<svelte:window onkeydown={closeOnEscape("stream")} />

<main class="stream">
  {#if engine.output}
    <ScaledCanvas fit="contain" background={engine.output.stream.backdrop}>
      <StreamView output={engine.output} />
    </ScaledCanvas>
  {/if}
</main>

<style>
  .stream {
    position: fixed;
    inset: 0;
    background: #000;
    cursor: none;
    user-select: none;
  }
</style>
