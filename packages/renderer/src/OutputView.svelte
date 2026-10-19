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
  the background without any text.

  - `muted`: silence videos (the operator's live pane), so only the
    projector plays sound.
-->
<script lang="ts">
  import type { OutputState, RenderFrame } from "@simpleworship/protocol";
  import { fade } from "svelte/transition";
  import BackgroundLayer from "./BackgroundLayer.svelte";
  import { getMediaUrls } from "./media.ts";
  import TextLayer from "./TextLayer.svelte";

  interface Props {
    output: OutputState;
    muted?: boolean;
  }

  let { output, muted = false }: Props = $props();

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

  const textLayers: Layer[] = $derived(
    output.frame && !flags.hideText && !flags.logo
      ? [{ key: textKey(output.frame), frame: output.frame }]
      : [],
  );

  /** Everything that affects how the text layer looks. */
  function textKey(frame: RenderFrame): string {
    const { text, scripture, caption, footer, theme } = frame;
    return JSON.stringify([
      text,
      scripture,
      caption,
      footer,
      theme.text,
      theme.textBox,
      theme.footer,
    ]);
  }
</script>

{#each backgroundLayers as layer (layer.key)}
  <div class="layer" transition:fade={{ duration }}>
    <BackgroundLayer background={layer.frame.theme.background} playback={output.video} {muted} />
  </div>
{/each}

{#each textLayers as layer (layer.key)}
  <div class="layer" transition:fade={{ duration }}>
    <TextLayer frame={layer.frame} />
  </div>
{/each}

{#if showLogo && output.logo}
  <div class="layer logo" transition:fade={{ duration }}>
    <img src={urls.file(output.logo)} alt="">
  </div>
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

  .logo img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
</style>
