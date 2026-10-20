<!--
  @component
  Release notes, drawn from the Markdown in CHANGELOG.md (see
  `lib/release-notes.ts`): headings, points and points under them, with
  bold, italic and code. Built from plain elements, never from HTML in the
  notes.
-->
<script lang="ts">
  import { type Inline, type NoteItem, parseNotes } from "../lib/release-notes.ts";

  interface Props {
    /** The notes, in Markdown. */
    markdown: string;
  }

  let { markdown }: Props = $props();

  const blocks = $derived(parseNotes(markdown));
</script>

{#snippet text(
  runs: Inline[],
)}
  {#each runs as run, index (index)}
    {#if run.code}
      <code>{run.text}</code>
    {:else if run.bold}
      <strong>{run.text}</strong>
    {:else if run.italic}
      <em>{run.text}</em>
    {:else}
      {run.text}
    {/if}
  {/each}
{/snippet}

{#snippet points(
  items: NoteItem[],
)}
  <ul>
    {#each items as item, index (index)}
      <li>
        {@render text(item.content)}
        {#if item.children.length > 0}
          {@render points(item.children)}
        {/if}
        {#if item.after}
          <p>{@render text(item.after)}</p>
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div class="notes">
  {#each blocks as block, index (index)}
    {#if block.type === "heading"}
      <h4>{@render text(block.content)}</h4>
    {:else if block.type === "paragraph"}
      <p>{@render text(block.content)}</p>
    {:else}
      {@render points(block.items)}
    {/if}
  {/each}
</div>

<style>
  .notes {
    line-height: 1.5;
  }

  h4 {
    margin: var(--sw-space-3) 0 var(--sw-space-1);
    font-size: var(--sw-font-size);
  }

  p {
    margin: 0 0 var(--sw-space-2);
  }

  ul {
    margin: 0 0 var(--sw-space-2);
    padding-left: 1.2em;
  }

  li {
    margin-bottom: var(--sw-space-1);
  }

  li > ul {
    margin: var(--sw-space-1) 0;
  }

  li > p {
    margin: var(--sw-space-1) 0 0;
    color: var(--sw-text-muted);
  }

  code {
    padding: 0 0.25em;
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    font-family: var(--sw-font-mono);
    font-size: 0.92em;
  }
</style>
