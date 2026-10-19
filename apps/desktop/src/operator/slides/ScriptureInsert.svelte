<!--
  @component
  Insert Scripture: type a reference ("acts 19:20"), choose the Bible, and
  the verses (and the reference) go into the selected text box, or a new
  one.
-->
<script lang="ts">
  import type { Passage } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { parseReference, scriptureText } from "../../lib/api.ts";
  import { bibles } from "../../lib/bibles.svelte.ts";

  interface Props {
    /** Called with the verse text and the reference (e.g. "Acts 19:20"). */
    oninsert: (verses: string, reference: string | null) => void;
    onclose: () => void;
  }

  let { oninsert, onclose }: Props = $props();

  let query = $state("");
  let bibleId: string | null = $state(bibles.list[0]?.id ?? null);
  let includeReference = $state(true);
  let passage: Passage | null = $state(null);
  let display = $state("");
  let preview = $state("");
  let problem: string | null = $state(null);
  let referenceInput: HTMLInputElement | null = $state(null);

  // The reference is the only thing to type.
  $effect(() => {
    referenceInput?.focus();
  });

  // Look the passage up as the reference is typed.
  $effect(() => {
    const text = query.trim();
    const id = bibleId;
    if (!text || !id) {
      preview = "";
      passage = null;
      return;
    }
    const timer = setTimeout(async () => {
      const feedback = await parseReference(text);
      problem = feedback.error;
      passage = feedback.passage;
      display = feedback.display;
      if (!feedback.passage) {
        preview = "";
        return;
      }
      try {
        const found = await scriptureText(feedback.passage, id);
        preview = found.verses.map((verse) => verse.text.trim()).join(" ");
        if (!preview) problem = "That passage is not in this Bible.";
      } catch (error) {
        problem = String(error);
        preview = "";
      }
    }, 200);
    return () => clearTimeout(timer);
  });

  function insert() {
    if (!passage || !preview) return;
    oninsert(preview, includeReference ? display : null);
  }
</script>

<Dialog title="Insert Scripture" size="small" {onclose}>
  <div class="form">
    <div class="row">
      <input
        bind:this={referenceInput}
        bind:value={query}
        placeholder="Reference, e.g. acts 19:20 or ps 91:15-16"
        aria-label="Reference"
        onkeydown={(e) => e.key === "Enter" && insert()}
      >
      <select bind:value={bibleId} aria-label="Bible">
        {#each bibles.list as bible (bible.id)}
          <option value={bible.id}>{bible.abbreviation}</option>
        {/each}
      </select>
    </div>
    {#if problem && query.trim()}
      <p class="problem">{problem}</p>
    {:else if preview}
      <p class="preview">{preview}</p>
      <p class="reference">{display}</p>
    {/if}
    <label class="check">
      <input type="checkbox" bind:checked={includeReference}>
      Add the reference on its own line
    </label>
  </div>

  {#snippet actions()}
    <Button onclick={onclose}>Cancel</Button>
    <Button variant="primary" disabled={!preview} onclick={insert}>Insert</Button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    min-width: 420px;
  }

  .row {
    display: flex;
    gap: var(--sw-space-2);
  }

  .row input {
    flex: 1;
  }

  .preview {
    margin: 0;
    max-height: 30vh;
    overflow-y: auto;
    line-height: 1.5;
  }

  .reference {
    margin: 0;
    color: var(--sw-text-muted);
    font-weight: 600;
  }

  .problem {
    margin: 0;
    color: var(--sw-live);
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }
</style>
