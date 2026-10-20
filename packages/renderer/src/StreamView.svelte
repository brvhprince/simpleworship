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
  });
</script>

<div class="backdrop" style:background={output.stream.backdrop}></div>
<OutputView output={shown} muted {still} />

<style>
  .backdrop {
    position: absolute;
    inset: 0;
  }
</style>
