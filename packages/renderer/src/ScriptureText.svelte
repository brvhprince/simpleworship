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
          <span class="number" style={numberStyle}>{line.number}</span>
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

  .translation {
    font-size: 0.45em;
    letter-spacing: 0.08em;
    opacity: 0.75;
    margin-bottom: 0.4em;
  }

  p {
    margin: 0;
  }

  .number {
    margin-right: 0.15em;
  }
</style>
