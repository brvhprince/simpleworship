<!--
  @component
  Draws the text of a slide on the canvas: song lyrics, or scripture in one
  or two columns. Below it sit the caption (e.g. "John 3:16–17 (KJV)") and
  the footer (copyright). The text shrinks to fit when the theme asks for it.
-->
<script lang="ts">
  import type { RenderFrame } from "@simpleworship/protocol";
  import { autofit } from "./autofit.ts";
  import CountdownText from "./CountdownText.svelte";
  import ScriptureText from "./ScriptureText.svelte";
  import { captionStyle, footerStyle, textBoxStyle, textStyle } from "./styles.ts";

  interface Props {
    frame: RenderFrame;
    /** Called when a live countdown on this slide reaches zero. */
    oncountdownend?: () => void;
  }

  let { frame, oncountdownend }: Props = $props();

  const theme = $derived(frame.theme);
  const showFooter = $derived(theme.footer.visible && !!frame.footer);
</script>

<div
  class="text-box"
  style={textBoxStyle(theme.textBox)}
  use:autofit={{
    enabled: theme.textBox.autoFit,
    maxSize: theme.text.fontSize,
    content: [frame.text, frame.scripture, frame.countdown, theme.text, theme.textBox],
  }}
>
  {#if frame.countdown}
    <div class="text" style={textStyle(theme.text)}>
      <CountdownText countdown={frame.countdown} onend={oncountdownend} />
    </div>
  {:else if frame.scripture}
    <div class="text columns" style={textStyle(theme.text)}>
      <ScriptureText scripture={frame.scripture} />
    </div>
  {:else}
    <div class="text" style={textStyle(theme.text)}>{frame.text}</div>
  {/if}
</div>

{#if frame.caption || showFooter}
  <div class="bottom" style:text-align={theme.text.align}>
    {#if frame.caption}
      <div style={captionStyle(theme)}>{frame.caption}</div>
    {/if}
    {#if showFooter}
      <div style={footerStyle(theme.footer, theme.text.align)}>{frame.footer}</div>
    {/if}
  </div>
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

  /* Scripture: translations side by side, sharing one font size. */
  .columns {
    display: flex;
    gap: 1.2em;
    white-space: normal;
  }

  .bottom {
    position: absolute;
    left: 120px;
    right: 120px;
    bottom: 40px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
