<!--
  @component
  Picks the screen for this window. Every SimpleWorship window loads the same
  page; the Rust side opens it with `?view=output` or `?view=stage`
  (see `src-tauri/src/windows.rs`). Without a view it is the operator window.
-->
<script lang="ts">
  import { setMediaResolver } from "@simpleworship/renderer";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import Operator from "./operator/Operator.svelte";
  import Output from "./output/Output.svelte";
  import Stage from "./stage/Stage.svelte";

  const view = new URLSearchParams(window.location.search).get("view") ?? "operator";

  // Media in themes are file paths; Tauri's asset protocol makes them loadable.
  setMediaResolver((path) => convertFileSrc(path));
</script>

{#if view === "output"}
  <Output />
{:else if view === "stage"}
  <Stage />
{:else}
  <Operator />
{/if}
