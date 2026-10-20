<!--
  @component
  The page an NDI source is drawn on (`?view=ndi&source=stream|main`), in a
  window nobody sees (see `src-tauri/src/ndi.rs`). It shows the stream look
  or the projector's picture, without fades (Rust blends the pictures
  instead), with videos and alerts as stills, on a see-through page.

  Whenever it has drawn something new, it tells Rust, which takes a
  picture of it: after each change to the output (and twice more shortly
  after, for whatever finishes drawing late), when a picture or font
  finishes loading, and every second while a countdown shows.
-->
<script lang="ts">
  import type { OutputState } from "@simpleworship/protocol";
  import { OutputView, ScaledCanvas, StreamView } from "@simpleworship/renderer";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";

  const source =
    new URLSearchParams(window.location.search).get("source") === "main" ? "main" : "stream";

  /** The output as drawn here: always a cut (Rust does the fading). */
  const shown: OutputState | null = $derived(
    engine.output ? { ...engine.output, transition: { kind: "cut", durationMs: 0 } } : null,
  );
  const background = $derived(
    source === "stream" ? (engine.output?.stream.backdrop ?? "transparent") : "#000",
  );
  const counting = $derived(
    Boolean(
      source === "stream"
        ? engine.output?.stream.frame?.countdown
        : engine.output?.frame?.countdown,
    ),
  );

  let drawnRevision = -1;

  /**
   * Waits until what changed is in the page, then asks for a picture.
   * Not for animation frames: a hidden window gets none. The picture is
   * taken after the page has been drawn (see `ndi_frame_ready`).
   */
  async function ready(fadeMs: number, refresh = false) {
    await tick();
    await Promise.all([...document.images].map((image) => image.decode().catch(() => {})));
    await invoke("ndi_frame_ready", { fadeMs, refresh }).catch(() => {});
  }

  /** Newer pictures of the same thing, a moment later. */
  function refreshSoon() {
    for (const delay of [400, 1500]) setTimeout(() => void ready(0, true), delay);
  }

  // A new output: a picture, fading as the projector does.
  $effect(() => {
    const output = engine.output;
    if (!output || output.revision === drawnRevision) return;
    const first = drawnRevision < 0;
    drawnRevision = output.revision;
    const fades = output.transition.kind === "fade" && !first;
    void ready(fades ? output.transition.durationMs : 0);
    refreshSoon();
  });

  // A countdown changes every second.
  $effect(() => {
    if (!counting) return;
    const timer = setInterval(() => void ready(0, true), 1000);
    return () => clearInterval(timer);
  });

  onMount(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    const connection = engine.connect();
    // Pictures and fonts that finish loading after a change.
    const loaded = (event: Event) => {
      if (event.target instanceof HTMLImageElement) void ready(0, true);
    };
    document.addEventListener("load", loaded, true);
    void document.fonts.ready.then(() => ready(0, true));
    return () => {
      document.removeEventListener("load", loaded, true);
      void connection.then((disconnect) => disconnect());
    };
  });
</script>

<main class="ndi">
  {#if shown}
    <ScaledCanvas fit="contain" {background}>
      {#if source === "stream"}
        <StreamView output={shown} still />
      {:else}
        <OutputView output={shown} muted still />
      {/if}
    </ScaledCanvas>
  {/if}
</main>

<style>
  .ndi {
    position: fixed;
    inset: 0;
    background: transparent;
  }
</style>
