<!--
  @component
  Draws the stream look of an `OutputState` (see `crates/sw-core/src/stream.rs`):
  the slide as the stream shows it, on the stream's backdrop, with the same
  transitions, video and alert as the main output. Wrap it in a
  `ScaledCanvas` with `background={output.stream.backdrop}`.

  The engine has already applied Black and Logo (they clear the stream), so
  nothing here covers the camera. Videos are always muted: the sound comes
  from the projector computer.
-->
<script lang="ts">
  import type { OutputState } from "@simpleworship/protocol";
  import { untrack } from "svelte";
  import OutputView from "./OutputView.svelte";

  interface Props {
    output: OutputState;
    /** Videos and alerts as stills (see `OutputView`). */
    still?: boolean;
  }

  let { output, still = false }: Props = $props();

  const shown: OutputState = $derived({
    ...output,
    frame: output.stream.frame,
    flags: {
      black: false,
      logo: false,
      hideText: output.stream.hideText,
      hideBackground: output.stream.hideBackground,
    },
    logo: null,
    alert: output.stream.showAlerts ? output.alert : null,
    // Black takes everything off the stream, lower thirds too.
    lowerThird: output.flags.black ? null : output.lowerThird,
  });

  /** Whether a theme's words are on the stream (the lower-third looks). */
  const wordsUp = $derived(
    output.stream.frame != null && !output.stream.hideText && output.stream.entrance != null,
  );
  /**
   * Counts the times the words appeared after nothing was showing, so each
   * plays the entrance once. Words already up when this view opened (OBS
   * connecting mid-song) are run 0: they show in place.
   */
  let entranceRun = $state(0);
  let wasUp = untrack(() => wordsUp);
  $effect.pre(() => {
    const up = wordsUp;
    if (up && !wasUp) entranceRun = untrack(() => entranceRun) + 1;
    wasUp = up;
  });
</script>

<div class="backdrop" style:background={output.stream.backdrop}></div>
<OutputView
  output={shown}
  muted
  {still}
  lowerThirds="stream"
  entrance={output.stream.entrance}
  {entranceRun}
/>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
  }
</style>
