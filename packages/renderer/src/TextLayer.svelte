<!--
  @component
  Draws the text of a slide (and its footer) on the canvas, shrinking the
  text to fit when the theme asks for it.
-->
<script lang="ts">
  import type { RenderFrame } from "@simpleworship/protocol";
  import { autofit } from "./autofit.ts";
  import { footerStyle, textBoxStyle, textStyle } from "./styles.ts";

  interface Props {
    frame: RenderFrame;
  }

  let { frame }: Props = $props();

  const theme = $derived(frame.theme);
  const showFooter = $derived(theme.footer.visible && !!frame.footer);
</script>

<div
  class="text-box"
  style={textBoxStyle(theme.textBox)}
  use:autofit={{
    enabled: theme.textBox.autoFit,
    maxSize: theme.text.fontSize,
    content: [frame.text, theme.text, theme.textBox],
  }}
>
  <div class="text" style={textStyle(theme.text)}>{frame.text}</div>
</div>

{#if showFooter}
  <div class="footer" style={footerStyle(theme.footer, theme.text.align)}>{frame.footer}</div>
{/if}

<style>
  .text-box {
    position: absolute;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .text {
    /* Keep the operator's line breaks; wrap long lines at spaces only, so a
       word that is too long makes the text shrink instead of breaking. */
    white-space: pre-line;
    overflow-wrap: normal;
  }

  .footer {
    position: absolute;
    left: 120px;
    right: 120px;
    bottom: 40px;
  }
</style>
