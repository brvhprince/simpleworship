<!--
  @component
  The verses of a scripture slide: one column per translation, with small
  raised verse numbers. Used inside `TextLayer`, which handles position,
  font and auto-fit.
-->
<script lang="ts">
  import type { ScriptureSlide } from "@simpleworship/protocol";

  interface Props {
    scripture: ScriptureSlide;
  }

  let { scripture }: Props = $props();

  const parallel = $derived(scripture.columns.length > 1);
</script>

{#each scripture.columns as column (column.translation)}
  <div class="column">
    {#if parallel}
      <div class="translation">{column.translation}</div>
    {/if}
    <p>
      {#each column.verses as line, index (index)}
        {#if scripture.verseNumbers && !line.continued}
          <sup class="number">{line.number}</sup>
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
    font-size: 0.55em;
    vertical-align: super;
    line-height: 0;
    margin-right: 0.15em;
    opacity: 0.8;
  }
</style>
