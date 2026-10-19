<!--
  @component
  The Scripture tab: find a passage and show it.

  - Type a reference ("jn 3:16-18") and press Enter to preview it; press
    Enter again to send it live. Or type words ("love is patient") to
    search the Bible.
  - Or pick a book, a chapter and verses. Click a verse to select it;
    shift-click another to select everything in between.
  - Choose a translation, and optionally a second one to show side by side.
-->
<script lang="ts">
  import type {
    BookOutline,
    Passage,
    ReferenceFeedback,
    ScriptureOptions,
    Verse,
  } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import {
    addScriptureToSchedule,
    bibleChapter,
    bibleOutline,
    parseReference,
    searchBible,
    showScriptureInPreview,
  } from "../../lib/api.ts";
  import { bibles } from "../../lib/bibles.svelte.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import ManageBibles from "./ManageBibles.svelte";

  /** Wait this long after the last keystroke before looking things up. */
  const TYPING_DELAY_MS = 120;

  /** Remembered between sessions on this computer (a convenience only). */
  const SETTINGS_KEY = "simpleworship.scripture";

  interface Settings {
    primaryId: string | null;
    secondaryId: string | null;
    maxVerses: number;
  }

  const saved = loadSettings();
  let primaryId: string | null = $state(saved.primaryId);
  let secondaryId: string | null = $state(saved.secondaryId);
  let maxVerses = $state(saved.maxVerses);
  /**
   * The quick verse-number switch: `null` follows the scripture theme
   * (set in the theme designer); a click overrides it until the theme's
   * setting is changed.
   */
  let verseNumbers = $state<boolean | null>(null);

  let query = $state("");
  let feedback = $state<ReferenceFeedback | null>(null);
  /** The selected passage, from typing or from the picker. */
  let passage = $state<Passage | null>(null);
  let outline: BookOutline[] = $state.raw([]);
  let chapterVerses: Verse[] = $state.raw([]);
  let searchResults: Verse[] = $state.raw([]);
  /** The verse a shift-click range starts from. */
  let anchorVerse: number | null = null;
  let managing = $state(false);

  const options: ScriptureOptions = $derived({ maxVerses, maxChars: 300, verseNumbers });

  /** Whether the scripture theme shows verse numbers. */
  const themeShowsNumbers = $derived.by(() => {
    const { themes, defaults } = engine.themes;
    const record = themes.find((theme) => theme.id === defaults.scripture) ?? themes[0];
    return record?.theme.scripture.verseNumbers.visible ?? true;
  });

  // A change to the theme's setting wins over an earlier quick switch.
  $effect(() => {
    void themeShowsNumbers;
    verseNumbers = null;
  });
  const bibleIds = $derived(
    [primaryId, secondaryId].filter((id): id is string => id !== null && id !== ""),
  );
  /** Words without numbers that are not a book name: search the text instead. */
  const searching = $derived(!!feedback?.error && query.trim() !== "" && !/\d/.test(query));
  const currentBook = $derived(outline.find((book) => book.book === passage?.book) ?? null);
  const currentChapter = $derived(passage?.ranges[0]?.start.chapter ?? null);

  // Pick a translation once the Bibles have loaded, or if the chosen one is gone.
  $effect(() => {
    const ids = bibles.list.map((bible) => bible.id);
    if (!primaryId || !ids.includes(primaryId)) primaryId = ids[0] ?? null;
    if (secondaryId && !ids.includes(secondaryId)) secondaryId = null;
  });

  $effect(() => saveSettings({ primaryId, secondaryId, maxVerses }));

  // Load the book list of the main translation.
  $effect(() => {
    const id = primaryId;
    outline = [];
    if (id) bibleOutline(id).then((books) => (outline = books), engine.showError.bind(engine));
  });

  // Understand what is typed: a reference, or words to search for.
  $effect(() => {
    const text = query;
    const timer = setTimeout(async () => {
      if (!text.trim()) {
        feedback = null;
        return;
      }
      feedback = await parseReference(text);
      if (feedback.passage) passage = feedback.passage;
    }, TYPING_DELAY_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const id = primaryId;
    const words = searching ? query : "";
    searchResults = [];
    if (!id || !words) return;
    const timer = setTimeout(async () => {
      searchResults = await searchBible(id, words);
    }, TYPING_DELAY_MS);
    return () => clearTimeout(timer);
  });

  // Show the chapter the selection is in.
  $effect(() => {
    const id = primaryId;
    const book = passage?.book;
    const chapter = currentChapter;
    chapterVerses = [];
    if (id && book && chapter) {
      bibleChapter(id, book, chapter).then((verses) => (chapterVerses = verses));
    }
  });

  // ── Picking with the mouse ─────────────────────────────────────────────

  function chooseBook(book: BookOutline) {
    choose(book, 1, null, null);
  }

  function chooseChapter(chapter: number) {
    if (currentBook) choose(currentBook, chapter, null, null);
  }

  function chooseVerse(verse: number, extend: boolean) {
    if (!currentBook || !currentChapter) return;
    if (extend && anchorVerse !== null) {
      choose(
        currentBook,
        currentChapter,
        Math.min(anchorVerse, verse),
        Math.max(anchorVerse, verse),
      );
    } else {
      anchorVerse = verse;
      choose(currentBook, currentChapter, verse, verse);
    }
  }

  /** Selects a whole chapter (no verses) or a verse range, and writes it in
   *  the reference box the way an operator would type it. */
  function choose(book: BookOutline, chapter: number, from: number | null, to: number | null) {
    passage = {
      book: book.book,
      ranges: [{ start: { chapter, verse: from }, end: { chapter, verse: to } }],
    };
    const verses = from === null ? "" : from === to ? `:${from}` : `:${from}-${to}`;
    query = `${book.name} ${chapter}${verses}`;
  }

  function chooseSearchResult(verse: Verse) {
    const book = outline.find((b) => b.book === verse.book);
    if (book) {
      anchorVerse = verse.verse;
      choose(book, verse.chapter, verse.verse, verse.verse);
    }
  }

  function isSelected(verse: number): boolean {
    return (
      passage?.ranges.some((range) => {
        const { start, end } = range;
        if (currentChapter === null) return false;
        const position = currentChapter * 1000 + verse;
        const first = start.chapter * 1000 + (start.verse ?? 0);
        const last = end.chapter * 1000 + (end.verse ?? 999);
        return position >= first && position <= last;
      }) ?? false
    );
  }

  // ── Showing ────────────────────────────────────────────────────────────

  /**
   * What was last sent to preview from here, as a key. A second Enter with
   * nothing changed sends it live, so a passage reaches the screen in two
   * keystrokes while the speaker is preaching.
   */
  let previewedKey: string | null = null;

  /** Identifies exactly what a preview would show. */
  function previewKey(shown: Passage): string {
    return JSON.stringify([shown, bibleIds, options]);
  }

  async function preview(shown: Passage | null = passage) {
    if (!shown) return;
    try {
      await showScriptureInPreview(shown, bibleIds, options);
      previewedKey = previewKey(shown);
    } catch (error) {
      engine.showError(error);
    }
  }

  /** Enter: preview the passage, or send it live if it is already previewed. */
  async function previewOrGoLive() {
    // Parse what is in the box right now: the as-you-type parse runs a
    // moment later, and a fast operator can press Enter before it does.
    const fresh = query.trim() ? (await parseReference(query)).passage : passage;
    if (!fresh) return;

    // Only go live if the preview still shows this passage: something else
    // (a song, a schedule item) may have been previewed in between.
    const source = engine.preview?.presentation.source;
    const stillPreviewed =
      previewKey(fresh) === previewedKey &&
      source?.type === "scripture" &&
      JSON.stringify(source.passage) === JSON.stringify(fresh);
    if (stillPreviewed) {
      engine.dispatch({ type: "goLive" });
    } else {
      passage = fresh;
      await preview(fresh);
    }
  }

  async function addToSchedule() {
    if (!passage) return;
    try {
      await addScriptureToSchedule(passage, bibleIds, options);
    } catch (error) {
      engine.showError(error);
    }
  }

  function handleQueryKeys(event: KeyboardEvent) {
    if (event.key === "Enter" && !searching) {
      event.preventDefault();
      void previewOrGoLive();
    }
  }

  // ── Settings ───────────────────────────────────────────────────────────

  function loadSettings(): Settings {
    const defaults: Settings = {
      primaryId: null,
      secondaryId: null,
      maxVerses: 2,
    };
    try {
      const stored = JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "{}");
      return { ...defaults, ...stored };
    } catch {
      return defaults;
    }
  }

  function saveSettings(settings: Settings) {
    try {
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    } catch {
      // Storage can be unavailable; the settings just won't be remembered.
    }
  }

  function reference(verse: Verse): string {
    const name = outline.find((book) => book.book === verse.book)?.name ?? "";
    return `${name} ${verse.chapter}:${verse.verse}`;
  }
</script>

<div class="scripture-tab">
  <div class="toolbar">
    <select bind:value={primaryId} aria-label={t("scripture.translation")}>
      {#each bibles.list as bible (bible.id)}
        <option value={bible.id}>{bible.abbreviation}</option>
      {/each}
    </select>
    <select bind:value={secondaryId} aria-label={t("scripture.second")}>
      <option value={null}>{t("scripture.noParallel")}</option>
      {#each bibles.list.filter((b) => b.id !== primaryId) as bible (bible.id)}
        <option value={bible.id}>+ {bible.abbreviation}</option>
      {/each}
    </select>

    <div class="reference">
      <input
        type="search"
        bind:value={query}
        onkeydown={handleQueryKeys}
        placeholder={t("scripture.placeholder")}
        aria-label={t("scripture.search")}
        aria-describedby="reference-feedback"
      >
      <span id="reference-feedback" class="feedback" class:error={feedback?.error && !searching}>
        {#if searching}
          {t("scripture.searching", { words: query.trim() })}
        {:else if feedback?.error}
          {feedback.error}
        {:else if feedback}
          {feedback.display}
          {#if feedback.alternatives.length}
            <span class="alternatives">
              {t("scripture.alternatives", { names: feedback.alternatives.slice(0, 3).join(", ") })}
            </span>
          {/if}
        {/if}
      </span>
    </div>

    <label class="option">
      {t("scripture.versesPerSlide")}
      <select bind:value={maxVerses}>
        {#each [1, 2, 3, 4, 6] as count (count)}
          <option value={count}>{count}</option>
        {/each}
      </select>
    </label>
    <label class="option" title={t("scripture.numbersHint")}>
      <input
        type="checkbox"
        checked={verseNumbers ?? themeShowsNumbers}
        onchange={(e) => {
          const on = e.currentTarget.checked;
          verseNumbers = on === themeShowsNumbers ? null : on;
        }}
      >
      {t("scripture.verseNumbers")}
    </label>

    <span class="spacer"></span>
    <Button onclick={() => (managing = true)}>{t("scripture.bibles")}</Button>
    <Button disabled={!passage || bibleIds.length === 0} onclick={addToSchedule}>
      {t("common.addToSchedule")}
    </Button>
    <Button
      variant="primary"
      disabled={!passage || bibleIds.length === 0}
      onclick={() => preview()}
    >
      {t("common.showInPreview")}
    </Button>
  </div>

  {#if bibles.loaded && bibles.list.length === 0}
    <p class="empty">
      {t("scripture.preparing")}
    </p>
  {:else}
    <div class="picker">
      <ul class="books" aria-label={t("scripture.books")}>
        {#each outline as book (book.book)}
          <li>
            <button
              type="button"
              class:selected={book.book === passage?.book}
              onclick={() => chooseBook(book)}
            >
              {book.name}
            </button>
          </li>
        {/each}
      </ul>

      <fieldset class="chapters" aria-label={t("scripture.chapters")}>
        {#if currentBook}
          {#each currentBook.chapters as verseCount, index (index)}
            {#if verseCount > 0}
              <button
                type="button"
                class:selected={index + 1 === currentChapter}
                onclick={() => chooseChapter(index + 1)}
              >
                {index + 1}
              </button>
            {/if}
          {/each}
        {/if}
      </fieldset>

      <ol class="verses" aria-label={searching ? t("scripture.results") : t("scripture.verses")}>
        {#if searching}
          {#each searchResults as verse (`${verse.book}.${verse.chapter}.${verse.verse}`)}
            <li>
              <button type="button" onclick={() => chooseSearchResult(verse)}>
                <span class="number">{reference(verse)}</span>
                {verse.text}
              </button>
            </li>
          {:else}
            <li class="hint">{t("scripture.noResults")}</li>
          {/each}
        {:else}
          {#each chapterVerses as verse (verse.verse)}
            <li>
              <button
                type="button"
                class:selected={isSelected(verse.verse)}
                onclick={(event) => chooseVerse(verse.verse, event.shiftKey)}
              >
                <span class="number">{verse.verse}</span>
                {verse.text}
              </button>
            </li>
          {:else}
            <li class="hint">{t("scripture.pick")}</li>
          {/each}
        {/if}
      </ol>
    </div>
  {/if}
</div>

{#if managing}
  <ManageBibles onclose={() => (managing = false)} />
{/if}

<style>
  .scripture-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
  }

  .reference {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .reference input {
    width: 300px;
  }

  .feedback {
    min-height: 1.2em;
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
  }

  .feedback.error {
    color: var(--sw-warning);
  }

  .alternatives {
    color: var(--sw-text-faint);
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .spacer {
    flex: 1;
  }

  .empty {
    margin: var(--sw-space-4);
    color: var(--sw-text-muted);
  }

  .picker {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 180px 200px 1fr;
    border-top: 1px solid var(--sw-border);
  }

  .books,
  .verses {
    list-style: none;
    margin: 0;
    padding: var(--sw-space-1) 0;
    overflow-y: auto;
  }

  .books,
  .chapters {
    border-right: 1px solid var(--sw-border);
  }

  .chapters {
    margin: 0;
    border: none;
    border-right: 1px solid var(--sw-border);
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 2px;
    padding: var(--sw-space-2);
    overflow-y: auto;
  }

  button {
    border: none;
    background: none;
    cursor: pointer;
    text-align: left;
    border-radius: var(--sw-radius-small);
  }

  button:hover {
    background: var(--sw-surface-hover);
  }

  button.selected {
    background: var(--sw-accent);
  }

  .books button {
    width: 100%;
    padding: 3px var(--sw-space-3);
  }

  .chapters button {
    width: 34px;
    padding: 4px 0;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .verses button {
    width: 100%;
    padding: var(--sw-space-1) var(--sw-space-3);
    line-height: 1.45;
  }

  .number {
    margin-right: var(--sw-space-2);
    color: var(--sw-text-muted);
    font-variant-numeric: tabular-nums;
  }

  .selected .number {
    color: inherit;
  }

  .hint {
    padding: var(--sw-space-3);
    color: var(--sw-text-faint);
  }
</style>
