<!--
  @component
  Create or edit a song. Song details and lyrics are on the left; on the
  right, a live preview shows how the lyrics will be split into slides.

  ⌘/Ctrl+S saves, Escape cancels.
-->
<script lang="ts">
  import type { Presentation, Song } from "@simpleworship/protocol";
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { untrack } from "svelte";
  import { saveSong, songPresentation } from "../../lib/api.ts";
  import { frameAt } from "../../lib/frames.ts";
  import { withModifier } from "../../lib/platform.ts";

  interface Props {
    /** The song to edit. A song with `createdAt === 0` is new. */
    song: Song;
    onsave: (saved: Song) => void;
    oncancel: () => void;
  }

  let { song, onsave, oncancel }: Props = $props();

  /** Wait this long after the last keystroke before re-splitting the slides. */
  const PREVIEW_DELAY_MS = 250;

  // The editor works on a copy, so Cancel really discards the changes.
  // `untrack` makes it explicit that the copy is taken once, when the
  // editor opens, and does not follow later changes to the `song` prop.
  let draft: Song = $state(untrack(() => ({ ...song })));
  let slides: Presentation | null = $state(null);
  let saving = $state(false);
  let error: string | null = $state(null);

  const isNew = untrack(() => song.createdAt === 0);

  let titleInput: HTMLInputElement;

  // Put the cursor in the title field as soon as the editor opens.
  $effect(() => {
    titleInput.focus();
  });

  // Re-split the lyrics into slides as the operator types.
  $effect(() => {
    const snapshot = $state.snapshot(draft);
    const timer = setTimeout(async () => {
      try {
        slides = await songPresentation(snapshot);
      } catch {
        // The preview is a convenience; saving reports real problems.
      }
    }, PREVIEW_DELAY_MS);
    return () => clearTimeout(timer);
  });

  async function save() {
    if (!draft.title.trim()) {
      error = "Please give the song a title.";
      return;
    }
    saving = true;
    error = null;
    try {
      onsave(await saveSong($state.snapshot(draft)));
    } catch (failure) {
      error = String(failure);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "s" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      save();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<Dialog title={isNew ? "New Song" : `Edit “${draft.title}”`} onclose={oncancel}>
  <div class="editor">
    <div class="form">
      <label class="wide">
        Title
        <input bind:this={titleInput} bind:value={draft.title}>
      </label>
      <label>
        Author
        <input bind:value={draft.author}>
      </label>
      <label>
        CCLI song number
        <input bind:value={draft.ccliNumber} inputmode="numeric">
      </label>
      <label class="wide">
        Copyright
        <input bind:value={draft.copyright} placeholder="© 2024 Example Music">
      </label>
      <label class="wide">
        Lyrics
        <textarea bind:value={draft.lyrics} spellcheck="true"></textarea>
      </label>
      <p class="help wide">
        Start each section with a tag on its own line, such as <code>[Verse 1]</code>,
        <code>[Chorus]</code>
        or <code>[Bridge]</code>. A blank line starts a new slide.
      </p>
      <label class="wide">
        Order (optional)
        <input bind:value={draft.arrangement} placeholder="V1 C V2 C B C">
      </label>
    </div>

    <aside class="slides" aria-label="Slide preview">
      <h3>{slides?.slides.length ?? 0} slides</h3>
      {#if slides}
        {#each slides.slides as slide, index (index)}
          {@const frame = frameAt(slides, index)}
          {#if slide.startsSection && slide.section}
            <span class="section">{slide.section.name}</span>
          {/if}
          {#if frame}
            <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
          {/if}
        {/each}
      {/if}
    </aside>
  </div>

  {#snippet actions()}
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <Button onclick={oncancel}>Cancel</Button>
    <Button variant="primary" disabled={saving} shortcut={withModifier("S")} onclick={save}
      >Save</Button
    >
  {/snippet}
</Dialog>

<style>
  .editor {
    display: grid;
    grid-template-columns: 1fr 280px;
    gap: var(--sw-space-4);
    height: min(70vh, 720px);
  }

  .form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: repeat(3, auto) 1fr repeat(2, auto);
    gap: var(--sw-space-3);
    min-height: 0;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .wide {
    grid-column: 1 / -1;
  }

  input,
  textarea {
    color: var(--sw-text);
  }

  textarea {
    flex: 1;
    min-height: 200px;
    resize: none;
    font-family: var(--sw-font-mono);
    line-height: 1.5;
  }

  .help {
    margin: 0;
    color: var(--sw-text-faint);
  }

  code {
    font-family: var(--sw-font-mono);
    color: var(--sw-text-muted);
  }

  .slides {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    overflow-y: auto;
    padding-right: var(--sw-space-1);
  }

  h3 {
    margin: 0;
    font-size: var(--sw-font-size);
    color: var(--sw-text-muted);
  }

  .section {
    margin-top: var(--sw-space-2);
    font-weight: 600;
  }

  .error {
    color: var(--sw-live);
    margin-right: auto;
  }
</style>
