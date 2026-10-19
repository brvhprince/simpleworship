<!--
  @component
  A still picture of one slide: background plus text, with no transitions.
  Used for preview panes and thumbnails. Wrap it in a `ScaledCanvas`.
-->
<script lang="ts">
  import type { RenderFrame } from "@simpleworship/protocol";
  import BackgroundLayer from "./BackgroundLayer.svelte";
  import TextLayer from "./TextLayer.svelte";

  interface Props {
    frame: RenderFrame;
    showText?: boolean;
    showBackground?: boolean;
  }

  let { frame, showText = true, showBackground = true }: Props = $props();
</script>

{#if showBackground}
  <BackgroundLayer background={frame.theme.background} />
{/if}
{#if showText}
  <TextLayer {frame} />
{/if}
