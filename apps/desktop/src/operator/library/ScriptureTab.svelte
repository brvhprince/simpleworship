<!--
  @component
  The Scripture tab: find a passage and show it.

  - Type a reference ("jn 3:16-18") and press Enter to preview it; press
    Enter again to send it live. Or type words ("love is patient") to
    search the Bible.
  - Or pick a book, a chapter and verses. Click a verse to show it in the
    preview; shift-click another to show everything in between;
    double-click to send it live. Clicking a search result previews it and
    opens its chapter, where the same works.
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
  import { assistant } from "../../lib/assistant.svelte.ts";
  import { bibles } from "../../lib/bibles.svelte.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { link } from "../../lib/link.svelte.ts";
  import { type Heard, listen } from "../../lib/listen.svelte.ts";
  import { scriptureState } from "../../lib/scripture-state.svelte.ts";
  import HeardStrip from "./HeardStrip.svelte";
  import ManageBibles from "./ManageBibles.svelte";

  /** Wait this long after the last keystroke before looking things up. */
  const TYPING_DELAY_MS = 120;

  let primaryId: string | null = $state(scriptureState.primaryId);
  let secondaryId: string | null = $state(scriptureState.secondaryId);
  let maxVerses = $state(scriptureState.maxVerses);
  let autoFlow = $state(scriptureState.autoFlow);
  let minFontSize = $state(scriptureState.minFontSize);
  let verseNumbers = $state<boolean | null>(scriptureState.verseNumbers);

  let query = $state(scriptureState.query);
  let feedback = $state<ReferenceFeedback | null>(scriptureState.feedback);
  /** The selected passage, from typing or from the picker. */
  let passage = $state<Passage | null>(scriptureState.passage);
  let outline: BookOutline[] = $state.raw(scriptureState.outline);
  let chapterVerses: Verse[] = $state.raw(scriptureState.chapterVerses);
  let searchResults: Verse[] = $state.raw(scriptureState.searchResults);
  /** The verse a shift-click range starts from. */
  let anchorVerse: number | null = null;
  let managing = $state(false);
  /** Passages the assistant suggested for the words typed. */
  let ideas: {
    about: string;
    list: { reference: string; why: string; passage: Passage }[];
  } | null = $state(scriptureState.ideas);

  // Sync to persistent singleton store so tab switches retain state.
  $effect(() => {
    scriptureState.primaryId = primaryId;
    scriptureState.secondaryId = secondaryId;
    scriptureState.maxVerses = maxVerses;
    scriptureState.autoFlow = autoFlow;
    scriptureState.minFontSize = minFontSize;
    scriptureState.verseNumbers = verseNumbers;
    scriptureState.query = query;
    scriptureState.feedback = feedback;
    scriptureState.passage = passage;
    scriptureState.outline = outline;
    scriptureState.chapterVerses = chapterVerses;
    scriptureState.searchResults = searchResults;
    scriptureState.ideas = ideas;
    scriptureState.saveSettings();
  });

  const options: ScriptureOptions = $derived({
    maxVerses,
    maxChars: 300,
    verseNumbers,
    autoFlow,
    minFontSize,
  });

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
  /** The selection's book, as a plain number: effects that depend on it
   *  rerun only when the book changes, not on every new selection. */
  const currentBookId = $derived(passage?.book ?? null);

  // Pick a translation once the Bibles have loaded, or if the chosen one is gone.
  $effect(() => {
    const ids = bibles.list.map((bible) => bible.id);
    if (!primaryId || !ids.includes(primaryId)) primaryId = ids[0] ?? null;
    if (secondaryId && !ids.includes(secondaryId)) secondaryId = null;
  });

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

  // Show the chapter the selection is in. Only a different book or chapter
  // reloads it: choosing verses in it must keep the same list on screen
  // (or a double-click would land on a new list and never count).
  $effect(() => {
    const id = primaryId;
    const book = currentBookId;
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
      // If the clicked verse is already part of a multi-verse range, preserve the range
      // (prevents double-clicking to go live from collapsing the range to a single verse).
      const hasRange =
        passage &&
        passage.ranges.some(
          (r) =>
            r.start.chapter !== r.end.chapter ||
            (r.start.verse !== null && r.start.verse !== r.end.verse),
        );
      if (hasRange && isSelected(verse)) {
        anchorVerse = verse;
        void preview(passage);
        return;
      }
      anchorVerse = verse;
      choose(currentBook, currentChapter, verse, verse);
    }
    // A click on a verse shows it (or the range) in the preview at once.
    void preview(passage);
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
      void preview(passage);
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

  /** Shows a passage in the preview. Resolves to whether it worked. */
  async function preview(shown: Passage | null = passage): Promise<boolean> {
    if (!shown) return false;
    try {
      await showScriptureInPreview(shown, bibleIds, options);
      previewedKey = previewKey(shown);
      return true;
    } catch (error) {
      engine.showError(error);
      return false;
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

  /** A reference Listen heard: select it and show it in the preview. */
  function showHeard(heard: Heard): Promise<boolean> {
    query = heard.reference;
    passage = heard.passage;
    return preview(heard.passage);
  }

  /** A double-click on a heard reference: straight to live. */
  async function sendHeardLive(heard: Heard) {
    query = heard.reference;
    passage = heard.passage;
    await sendLive(heard.passage);
  }

  /**
   * A double-click (on a verse in the chapter, or a heard reference):
   * straight to live. The clicks before it have already started the
   * preview; showing it again first makes sure live gets this passage, not
   * whatever was previewed before.
   */
  async function sendLive(shown: Passage | null = passage) {
    if (await preview(shown)) engine.dispatch({ type: "goLive" });
  }

  // The H key, pressed anywhere: the newest reference heard.
  $effect(() => {
    const requested = listen.request;
    if (!requested) return;
    listen.request = null;
    void showHeard(requested);
  });

  // Looking at the tab sees what was heard.
  $effect(() => {
    void listen.heard;
    listen.unseen = 0;
  });

  /** Ask AI: passages on the theme typed ("God's faithfulness in trouble"). */
  async function askAssistant() {
    const about = query.trim();
    try {
      const found = await assistant.findVerses(about);
      const list = [];
      for (const idea of found) {
        // Only passages that really parse (the assistant can be wrong).
        const parsed = await parseReference(idea.reference);
        if (parsed.passage) list.push({ ...idea, passage: parsed.passage });
      }
      if (query.trim() === about) ideas = { about, list };
    } catch (error) {
      engine.showError(error);
    }
  }

  /** A suggested passage: select it and show it in the preview. */
  function chooseIdea(idea: { reference: string; passage: Passage }) {
    query = idea.reference;
    passage = idea.passage;
    void preview(idea.passage);
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

  function reference(verse: Verse): string {
    const name = outline.find((book) => book.book === verse.book)?.name ?? "";
    return `${name} ${verse.chapter}:${verse.verse}`;
  }
</script>

<div class="scripture-tab">
  <HeardStrip onchoose={showHeard} ongolive={sendHeardLive} />
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
      <!-- What was understood ("John 3:16–18") shows inside the box, on the
           right, so it takes no room of its own. -->
      <span class="field">
        <input
          type="search"
          bind:value={query}
          onkeydown={handleQueryKeys}
          placeholder={t("scripture.placeholder")}
          aria-label={t("scripture.search")}
          aria-describedby="reference-feedback"
          class:typed={query.trim() !== ""}
        >
        <span
          id="reference-feedback"
          class="feedback"
          class:error={feedback?.error && !searching}
          title={searching ? undefined : (feedback?.error ?? feedback?.display)}
        >
          {#if !query.trim()}
            <!-- Nothing typed: the placeholder says what to do. -->
          {:else if searching}
            {t("scripture.searching", { words: query.trim() })}
          {:else if feedback?.error}
            {feedback.error}
          {:else if feedback}
            {feedback.display}
            {#if feedback.alternatives.length}
              <span class="alternatives">
                {t("scripture.alternatives", {
                  names: feedback.alternatives.slice(0, 3).join(", "),
                })}
              </span>
            {/if}
          {/if}
        </span>
      </span>
      {#if searching && assistant.ready}
        <Button
          disabled={assistant.busy}
          title={t("assistant.findVersesHint")}
          onclick={askAssistant}
        >
          {assistant.busy ? t("assistant.thinking") : t("assistant.ask")}
        </Button>
      {/if}
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
    <label class="option" title={t("scripture.autoFlowHint")}>
      <input type="checkbox" bind:checked={autoFlow}>
      {t("scripture.autoFlow")}
    </label>
    {#if autoFlow}
      <label class="option" title={t("scripture.minFontSizeHint")}>
        {t("scripture.minFontSize")}
        <input
          type="number"
          min="16"
          max="120"
          class="min-font-size"
          bind:value={minFontSize}
        >
      </label>
    {/if}

    <span class="spacer"></span>
    {#if !link.joined}
      <Button onclick={() => (managing = true)}>{t("scripture.bibles")}</Button>
    {/if}
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
          {#if ideas && ideas.about === query.trim()}
            <li class="hint">{t("assistant.versesFor", { about: ideas.about })}</li>
            {#each ideas.list as idea (idea.reference)}
              <li>
                <button type="button" onclick={() => chooseIdea(idea)}>
                  <span class="number">{idea.reference}</span>
                  {idea.why}
                </button>
              </li>
            {:else}
              <li class="hint">{t("assistant.nothingFound")}</li>
            {/each}
            <li class="hint">{t("assistant.wordsInBible")}</li>
          {/if}
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
                ondblclick={() => sendLive()}
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
    align-items: center;
    gap: var(--sw-space-2);
  }

  /* The box, with what was understood written faintly at its right end. */
  .field {
    position: relative;
    display: flex;
  }

  .field input {
    width: 420px;
  }

  /* The system's clear button would sit in the middle of the box, beside
     the feedback; Escape or typing over the selection clears it. */
  .field input::-webkit-search-cancel-button {
    display: none;
  }

  /* Room for the feedback once something is typed (an empty box shows its
     whole placeholder). */
  .field input.typed {
    padding-right: 45%;
  }

  .feedback {
    position: absolute;
    top: 50%;
    right: var(--sw-space-2);
    max-width: 43%;
    overflow: hidden;
    transform: translateY(-50%);
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
    white-space: nowrap;
    text-overflow: ellipsis;
    pointer-events: none;
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

  .min-font-size {
    width: 52px;
    padding: 2px 4px;
    background: var(--sw-surface-raised);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    color: var(--sw-text);
  }
</style>
