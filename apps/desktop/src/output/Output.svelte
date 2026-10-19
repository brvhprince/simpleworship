<!--
  @component
  The projector output: the live slide, full screen, with nothing else.
  Opened by the Rust side on the chosen monitor (see `src-tauri/src/windows.rs`).
-->
<script lang="ts">
  import { OutputView, ScaledCanvas } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { closeOnEscape } from "../lib/output-window.ts";

  onMount(() => {
    const connection = engine.connect();
    return () => void connection.then((disconnect) => disconnect());
  });
</script>

<svelte:window onkeydown={closeOnEscape("projector")} />

<main class="output">
  {#if engine.output}
    <ScaledCanvas fit="contain">
      <OutputView output={engine.output} />
    </ScaledCanvas>
  {/if}
</main>

<style>
  .output {
    position: fixed;
    inset: 0;
    background: #000;
    /* The mouse pointer must never appear on the projector. */
    cursor: none;
    user-select: none;
  }
</style>
