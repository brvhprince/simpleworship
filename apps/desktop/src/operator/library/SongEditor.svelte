<!--
  @component
  Create or edit a song. Song details and lyrics are on the left; on the
  right, a live preview shows how the lyrics will be split into slides.

  ⌘/Ctrl+S saves, Escape cancels.

  With the assistant on, Tidy Lyrics sets out pasted lyrics in sections
  (with Undo), and Translate makes a new song in another language from
  this one, as a draft to correct; the original song is not changed.
-->
<script lang="ts">
  import type { Presentation, Song, TranslateTo } from "@simpleworship/protocol";
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { untrack } from "svelte";
  import { newSong, saveSong, songPresentation } from "../../lib/api.ts";
  import { assistant } from "../../lib/assistant.svelte.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { frameAt } from "../../lib/frames.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { overrideMenuItem } from "../../lib/menu.ts";
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

  let isNew = $state(untrack(() => song.createdAt === 0));
  /** The lyrics before Tidy Lyrics, for Undo. */
  let beforeTidy: string | null = $state(null);
  /** The song this one was translated from, if it was. */
  let translatedFrom: string | null = $state(null);

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

  /** Tidy Lyrics: sets out the lyrics in sections, and fills in any song
   *  details found in them that are still empty. */
  async function tidy() {
    error = null;
    try {
      const tidied = await assistant.tidyLyrics(draft.lyrics);
      beforeTidy = draft.lyrics;
      draft.lyrics = tidied.lyrics;
      draft.arrangement ||= tidied.arrangement;
      draft.title ||= tidied.title;
      draft.author ||= tidied.author;
      draft.copyright ||= tidied.copyright;
      draft.ccliNumber ||= tidied.ccliNumber;
    } catch (failure) {
      error = String(failure);
    }
  }

  function undoTidy() {
    if (beforeTidy === null) return;
    draft.lyrics = beforeTidy;
    beforeTidy = null;
  }

  /** Translate: this editor becomes a new song in that language. */
  async function translate(language: TranslateTo) {
    error = null;
    try {
      const translated = await assistant.translate(draft.title, draft.lyrics, language);
      translatedFrom = draft.title;
      const fresh = newSong();
      draft = {
        ...draft,
        id: fresh.id,
        createdAt: 0,
        updatedAt: 0,
        title: translated.title,
        lyrics: translated.lyrics,
      };
      isNew = true;
      beforeTidy = null;
    } catch (failure) {
      error = String(failure);
    }
  }

  async function save() {
    if (!draft.title.trim()) {
      error = t("song.needsTitle");
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

  // ⌘/Ctrl+S is the File menu's "Save Schedule" shortcut, and the menu
  // receives it before the page does. While the editor is open, the same
  // shortcut saves the song instead.
  $effect(() => overrideMenuItem("schedule-save", save));
</script>

<Dialog
  title={isNew ? t("song.newTitle") : t("song.editTitle", { title: draft.title })}
  onclose={oncancel}
>
  <div class="editor">
    <div class="form">
      <label class="wide">
        {t("song.title")}
        <input bind:this={titleInput} bind:value={draft.title}>
      </label>
      <label>
        {t("song.author")}
        <input bind:value={draft.author}>
      </label>
      <label>
        {t("song.ccli")}
        <input bind:value={draft.ccliNumber} inputmode="numeric">
      </label>
      <label class="wide">
        {t("song.copyright")}
        <input bind:value={draft.copyright} placeholder={t("song.copyrightPlaceholder")}>
      </label>
      <label class="wide">
        {t("song.lyrics")}
        <textarea bind:value={draft.lyrics} spellcheck="true"></textarea>
      </label>
      <div class="help wide">
        <p>
          {#if translatedFrom}
            {t("assistant.translatedFrom", { title: translatedFrom })}
          {:else}
            {t("song.help")}
          {/if}
        </p>
        {#if assistant.ready}
          <div class="assist">
            {#if beforeTidy !== null}
              <Button onclick={undoTidy}>{t("common.undo")}</Button>
            {/if}
            <Button disabled={assistant.busy || !draft.lyrics.trim()} onclick={tidy}>
              {assistant.busy ? t("assistant.thinking") : t("assistant.tidy")}
            </Button>
            <select
              aria-label={t("assistant.translate")}
              disabled={assistant.busy || !draft.lyrics.trim()}
              value=""
              onchange={(e) => {
                const language = e.currentTarget.value as TranslateTo;
                e.currentTarget.value = "";
                if (language) void translate(language);
              }}
            >
              <option value="" disabled>{t("assistant.translate")}</option>
              <option value="twi">Twi</option>
              <option value="french">Français</option>
              <option value="english">English</option>
            </select>
          </div>
        {/if}
      </div>
      <label>
        {t("song.order")}
        <input bind:value={draft.arrangement} placeholder="V1 C V2 C B C">
      </label>
      <label>
        {t("common.theme")}
        <select bind:value={draft.themeId}>
          <option value={null}>{t("song.defaultTheme")}</option>
          {#each engine.themes.themes as theme (theme.id)}
            <option value={theme.id}>{theme.theme.name}</option>
          {/each}
        </select>
      </label>
    </div>

    <aside class="slides" aria-label={t("song.slidePreview")}>
      <h3>{t("common.slides", { count: slides?.slides.length ?? 0 })}</h3>
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
    <Button onclick={oncancel}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={saving} shortcut={withModifier("S")} onclick={save}
      >{t("common.save")}</Button
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
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    color: var(--sw-text-faint);
  }

  .help p {
    flex: 1;
    margin: 0;
  }

  .assist {
    display: flex;
    flex: none;
    gap: var(--sw-space-1);
  }

  .slides {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    overflow-y: auto;
    padding-right: var(--sw-space-1);
  }

  /* Each slide keeps its 16:9 shape: in a scrolling column the pictures
     would otherwise shrink to fit, squashing the words. */
  .slides > :global(.viewport) {
    flex-shrink: 0;
    border-radius: var(--sw-radius-small);
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
