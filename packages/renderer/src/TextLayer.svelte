<!--
  @component
  Draws the text of a slide on the canvas: song lyrics, presentation text
  (with bold, italic and underline), scripture in one or two columns, or a
  countdown. The text shrinks to fit when the theme asks for it.

  The scripture reference ("John 3:16–17 (KJV)") and the footer (copyright)
  each sit in their own box, which the theme designer lets the operator
  drag, or straight after the text.
-->
<script lang="ts">
  import type { FooterStyle, RenderFrame } from "@simpleworship/protocol";
  import { autofit } from "./autofit.ts";
  import CountdownText from "./CountdownText.svelte";
  import ScriptureText from "./ScriptureText.svelte";
  import { labelStyle, textBoxStyle, textStyle } from "./styles.ts";

  interface Props {
    frame: RenderFrame;
    /** Called when a live countdown on this slide reaches zero. */
    oncountdownend?: () => void;
  }

  let { frame, oncountdownend }: Props = $props();

  const theme = $derived(frame.theme);

  /** The reference and the footer, each with the style it is drawn in. */
  const labels = $derived(
    [
      { key: "caption", text: frame.caption, style: theme.caption },
      { key: "footer", text: frame.footer, style: theme.footer },
    ].filter((label): label is { key: string; text: string; style: FooterStyle } =>
      Boolean(label.text && label.style.visible),
    ),
  );
  const afterText = $derived(labels.filter((label) => label.style.placement === "afterText"));
  const boxed = $derived(labels.filter((label) => label.style.placement === "box"));
</script>

<div
  class="text-box"
  style={textBoxStyle(theme.textBox)}
  use:autofit={{
    enabled: theme.textBox.autoFit,
    maxSize: theme.text.fontSize,
    scaleUp: theme.textBox.scaleUp,
    content: [frame.text, frame.rich, frame.scripture, frame.countdown, theme, afterText],
  }}
>
  <!-- One wrapper: autofit measures the text box's first child. -->
  <div class="content">
    {#if frame.countdown}
      <div class="text" style={textStyle(theme.text)}>
        <CountdownText countdown={frame.countdown} onend={oncountdownend} />
      </div>
    {:else if frame.scripture}
      <div class="text columns" style={textStyle(theme.text)}>
        <ScriptureText scripture={frame.scripture} style={theme.scripture} />
      </div>
    {:else if frame.rich}
      <div class="text" style={textStyle(theme.text)}>
        {#each frame.rich as run, index (index)}
          <span class:bold={run.bold} class:italic={run.italic} class:underline={run.underline}
            >{run.text}</span
          >
        {/each}
      </div>
    {:else}
      <div class="text" style={textStyle(theme.text)}>{frame.text}</div>
    {/if}

    {#each afterText as label (label.key)}
      <div class="label" style={labelStyle(label.style, theme.text, true)}>{label.text}</div>
    {/each}
  </div>
</div>

{#each boxed as label (label.key)}
  <div
    class="text-box"
    style={textBoxStyle(label.style.textBox)}
    use:autofit={{
      enabled: label.style.textBox.autoFit,
      maxSize: label.style.fontSize,
      content: [label.text, label.style, theme.text],
    }}
  >
    <div class="label" style={labelStyle(label.style, theme.text, false)}>{label.text}</div>
  </div>
{/each}

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

  .label {
    white-space: pre-line;
  }

  .content > .label:first-of-type {
    margin-top: 0.4em;
  }

  .bold {
    font-weight: bolder;
  }

  .italic {
    font-style: italic;
  }

  .underline {
    text-decoration: underline;
    text-decoration-thickness: 0.06em;
    text-underline-offset: 0.12em;
  }
</style>
