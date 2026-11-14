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
  sound (on `still` outputs, its first frame).

  - `muted`: silence videos (the operator's live pane), so only the
    projector plays sound.
  - `still`: videos as a still frame and alerts as a still banner, for
    outputs that are not redrawn often enough for movement (NDI).
  - `onalertend`, `oncountdownend`, `onlowerthirdend`: an alert ran its
    course, a countdown reached zero, a lower third's seconds are up (with
    the id of that showing, so the engine takes off only that one). The operator
    window passes these on to the engine. Every output also hides a timed
    lower third by itself when its time is up, so a screen that has lost
    touch with the main computer does not keep it.
  - `entrance`, `entranceRun`: how the slide's words come in when they
    appear after nothing was showing or a different item goes live (the
    stream's lower-third looks, see `StreamView`); each new run plays it
    once, while the words before fade out. The words' own transitions from
    slide to slide are unchanged.
  - `lowerThirds`: which lower thirds to show: those for the room's screens
    (projectors), the stream (`StreamView`), or all (the operator's live pane).

  Above the slides sit the lower third and then the alert (if any); Black
  covers everything.
-->
<script lang="ts">
  import type { OutputState, RenderFrame, StreamEntrance } from "@simpleworship/protocol";
  import { fade } from "svelte/transition";
  import AlertLayer from "./AlertLayer.svelte";
  import BackgroundLayer from "./BackgroundLayer.svelte";
  import ElementsLayer from "./ElementsLayer.svelte";
  import LowerThirdLayer from "./LowerThirdLayer.svelte";
  import { type LowerThirdPlace, lowerThirdFor, lowerThirdOut } from "./lower-thirds.ts";
  import { getMediaUrls } from "./media.ts";
  import TextLayer from "./TextLayer.svelte";

  interface Props {
    output: OutputState;
    muted?: boolean;
    still?: boolean;
    onalertend?: () => void;
    oncountdownend?: () => void;
    onlowerthirdend?: (showId: string) => void;
    lowerThirds?: LowerThirdPlace;
    entrance?: StreamEntrance | null;
    entranceRun?: number;
  }

  let {
    output,
    muted = false,
    still = false,
    onalertend,
    oncountdownend,
    onlowerthirdend,
    lowerThirds = "screens",
    entrance = null,
    entranceRun = 0,
  }: Props = $props();

  /** The showing of a timed lower third this output has seen run out. */
  let endedLowerThird: string | null = $state(null);
  const lowerThird = $derived.by(() => {
    const shown = lowerThirdFor(output.lowerThird, lowerThirds);
    return shown && shown.showId !== endedLowerThird ? shown : null;
  });

  /** A layer's timer ran out, for the showing it was mounted for. */
  function lowerThirdEnded(showId: string) {
    endedLowerThird = showId;
    onlowerthirdend?.(showId);
  }

  const urls = getMediaUrls();

  /** One version of a layer. `key` changes whenever the layer looks different. */
  interface Layer {
    key: string;
    frame: RenderFrame;
  }

  const duration = $derived(output.transition.kind === "cut" ? 0 : output.transition.durationMs);

  /**
   * Which entrance run the words belong to: 0 (never remounted) unless they
   * rise or wipe in, so the Fade entrance keeps the words' own fade.
   */
  const entranceKey = $derived(
    (entrance === "rise" || entrance === "wipe") && !still ? entranceRun : 0,
  );
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

<!-- A new entrance run remounts the words, so its entrance plays once
     (and the words before fade out); still outputs (NDI) and the first run
     (a screen opened late) skip it. -->
{#key entranceKey}
  <div
    class="layer words"
    class:rise={entrance === "rise" && entranceKey > 0}
    class:wipe={entrance === "wipe" && entranceKey > 0}
    out:fade={{ duration }}
  >
    {#each textLayers as layer (layer.key)}
      <div class="layer" transition:fade={{ duration }}>
        {#if layer.frame.elements}
          <ElementsLayer
            elements={layer.frame.elements}
            hideText={flags.hideText}
            {muted}
            {still}
          />
        {:else}
          <TextLayer frame={layer.frame} {oncountdownend} />
        {/if}
      </div>
    {/each}
  </div>
{/key}

{#if showLogo && output.logo}
  {@const logo = output.logo}
  <div class="layer logo" transition:fade={{ duration }}>
    {#if logo.kind === "video"}
      <!-- Still outputs show its first frame, paused (a media fragment, as
           ElementsLayer does), so they never depend on a thumbnail that may
           not have been made yet. -->
      <video
        src={still ? `${urls.file(logo.media)}#t=0.1` : urls.file(logo.media)}
        autoplay={!still}
        loop
        muted
        playsinline
        preload="auto"
      ></video>
    {:else}
      <img src={urls.file(logo.media)} alt="">
    {/if}
  </div>
{/if}

{#if lowerThird}
  {#key lowerThird.showId}
    <div class="layer" out:lowerThirdOut={{ style: lowerThird.item.look.style, still }}>
      <LowerThirdLayer shown={lowerThird} {still} onend={lowerThirdEnded} />
    </div>
  {/key}
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

  /* The stream's lower-third entrances (see `entrance`). */
  .words.rise {
    animation: words-rise 520ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }

  .words.wipe {
    animation: words-wipe 620ms cubic-bezier(0.65, 0, 0.35, 1) both;
  }

  @keyframes words-rise {
    from {
      opacity: 0;
      transform: translateY(48px);
    }
  }

  @keyframes words-wipe {
    from {
      clip-path: inset(0 100% 0 0);
    }
    to {
      clip-path: inset(0 0 0 0);
    }
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
