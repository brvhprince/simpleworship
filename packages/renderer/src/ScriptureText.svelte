<!--
  @component
  The verses of a scripture slide: one column per translation, with verse
  numbers styled by the theme. The Scripture tab can turn the numbers on or
  off for one passage (`scripture.verseNumbers`); otherwise the theme
  decides. Used inside `TextLayer`, which handles position, font and
  auto-fit.
-->
<script lang="ts">
  import type { ScriptureSlide, ScriptureStyle } from "@simpleworship/protocol";
  import { verseNumberStyle } from "./styles.ts";

  interface Props {
    scripture: ScriptureSlide;
    style: ScriptureStyle;
  }

  let { scripture, style }: Props = $props();

  const parallel = $derived(scripture.columns.length > 1);
  const showNumbers = $derived(scripture.verseNumbers ?? style.verseNumbers.visible);
  const numberStyle = $derived(verseNumberStyle(style.verseNumbers));
</script>

{#each scripture.columns as column (column.translation)}
  <div class="column">
    {#if parallel}
      <div class="translation">{column.translation}</div>
    {/if}
    <p>
      {#each column.verses as line, index (index)}
        {#if showNumbers && !line.continued}
          <!-- A thin space, not a margin: it scales with the number (see the styles). -->
          <span class="number" style={numberStyle}>{line.number}{"\u2009"}</span>
        {/if}
        {line.text}{" "}
      {/each}
    </p>
  </div>
{/each}

<style>
  .column {
    flex: 1 1 0;
    min-width: 0;
  }

  /*
   * No lengths in em here other than font sizes. The text is sized by
   * autofit, which changes the box's font size many times in a row, and
   * iOS 27's WebKit does not update em-based gaps, margins and letter
   * spacing for those changes: they kept the values of autofit's first,
   * largest try, which squeezed two-translation scripture into narrow
   * columns at the edges of the phone remote's preview. Spacing comes from
   * line height, percentages and characters instead.
   */
  .translation {
    font-size: 0.45em;
    line-height: 1.8;
    opacity: 0.75;
  }

  p {
    margin: 0;
  }
</style>
