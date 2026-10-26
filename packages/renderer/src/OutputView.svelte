<!--
  @component
  Draws an `OutputState` with animated transitions. This is what projector
  windows show. Wrap it in a `ScaledCanvas`.

  The output is drawn as separate layers, from bottom to top:

  1. **Background.** It only re-renders when the background itself changes,
     so a looping video keeps playing smoothly while the lyrics change on top.
  2. **Text** of the live slide.
  3. **Black**, which covers everything when the operator presses Black.

  Each layer cross-fades on its own: the outgoing and incoming versions are
  both on screen for the length of the transition.

  The Logo button shows the church logo on black, or, if no logo is set,
  the background without any text. A video logo plays on a loop, without
  sound (on `still` outputs, its thumbnail).

  - `muted`: silence videos (the operator's live pane), so only the
    projector plays sound.
  - `still`: videos as a still frame and alerts as a still banner, for
    outputs that are not redrawn often enough for movement (NDI).
  - `onalertend`, `oncountdownend`: an alert ran its course, a countdown
    reached zero. The operator window passes these on to the engine.

  Above the slides sits the alert (if any); Black covers everything.
-->
<script lang="ts">
  import type { OutputState, RenderFrame } from "@simpleworship/protocol";
  import { fade } from "svelte/transition";
  import AlertLayer from "./AlertLayer.svelte";
  import BackgroundLayer from "./BackgroundLayer.svelte";
  import ElementsLayer from "./ElementsLayer.svelte";
  import { getMediaUrls } from "./media.ts";
  import TextLayer from "./TextLayer.svelte";

  interface Props {
    output: OutputState;
    muted?: boolean;
    still?: boolean;
    onalertend?: () => void;
    oncountdownend?: () => void;
  }

  let { output, muted = false, still = false, onalertend, oncountdownend }: Props = $props();

  const urls = getMediaUrls();

  /** One version of a layer. `key` changes whenever the layer looks different. */
  interface Layer {
    key: string;
    frame: RenderFrame;
  }

  const duration = $derived(output.transition.kind === "cut" ? 0 : output.transition.durationMs);
  const flags = $derived(output.flags);

  // Each layer is a list of zero or one item. Rendering it with a keyed
  // `#each` means that when the key changes, the old item keeps its own data
  // while it fades out, instead of fading out with the new text already in it.
  const backgroundLayers: Layer[] = $derived(
    output.frame && !flags.hideBackground
      ? [{ key: JSON.stringify(output.frame.theme.background), frame: output.frame }]
      : [],
  );

  const showLogo = $derived(flags.logo && output.logo !== null);

  // A canvas-editor slide keeps its shapes and pictures when the text is
  // cleared, so its layer stays and only leaves its text out.
  const textLayers: Layer[] = $derived(
    output.frame && !flags.logo && (output.frame.elements || !flags.hideText)
      ? [{ key: textKey(output.frame), frame: output.frame }]
      : [],
  );

  /** Everything that affects how the text layer looks. */
  function textKey(frame: RenderFrame): string {
    const { text, rich, scripture, caption, footer, theme } = frame;
    if (frame.elements) return JSON.stringify([frame.elements, flags.hideText]);
    return JSON.stringify([
      text,
      rich,
      scripture,
      caption,
      footer,
      theme.text,
      theme.textBox,
      theme.footer,
      theme.caption,
      theme.scripture,
    ]);
  }
</script>

{#each backgroundLayers as layer (layer.key)}
  <div class="layer" transition:fade={{ duration }}>
    <BackgroundLayer
      background={layer.frame.theme.background}
      playback={output.video}
      {muted}
      {still}
    />
  </div>
{/each}

{#each textLayers as layer (layer.key)}
  <div class="layer" transition:fade={{ duration }}>
    {#if layer.frame.elements}
      <ElementsLayer elements={layer.frame.elements} hideText={flags.hideText} {muted} {still} />
    {:else}
      <TextLayer frame={layer.frame} {oncountdownend} />
    {/if}
  </div>
{/each}

{#if showLogo && output.logo}
  {@const logo = output.logo}
  <div class="layer logo" transition:fade={{ duration }}>
    {#if logo.kind === "video" && !still}
      <video src={urls.file(logo.media)} autoplay loop muted playsinline preload="auto"></video>
    {:else if logo.kind === "video"}
      <img src={urls.thumbnail(logo.media)} alt="">
    {:else}
      <img src={urls.file(logo.media)} alt="">
    {/if}
  </div>
{/if}

{#if output.alert}
  {#key output.alert.startedAt}
    <div class="layer" transition:fade={{ duration: still ? 0 : 300 }}>
      <AlertLayer alert={output.alert} {still} onend={onalertend} />
    </div>
  {/key}
{/if}

{#if flags.black}
  <div class="layer black" transition:fade={{ duration }}></div>
{/if}

<style>
  .layer {
    position: absolute;
    inset: 0;
  }

  .black,
  .logo {
    background: #000;
  }

  .logo img,
  .logo video {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
</style>
