<!--
  @component
  Scripture from the phone, as fast as at the desk: type the reference the
  way you would say it ("isa 29:12-13, 17-21"), Enter to preview, Enter
  again to send it live.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import type { BibleInfo, Passage, ReferenceFeedback } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { remote } from "../lib/remote.svelte.ts";
  import PreviewCard from "./PreviewCard.svelte";

  /** The translation chosen here, remembered on this phone. */
  const BIBLE_KEY = "simpleworship.remote.bible";

  let bibles: BibleInfo[] = $state([]);
  let bibleId: string | null = $state(null);
  let query = $state("");
  let feedback: ReferenceFeedback | null = $state(null);
  /** The passage last sent to preview, so a second Enter sends it live. */
  let previewed: Passage | null = $state(null);

  onMount(async () => {
    bibles = await remote.call<BibleInfo[]>("bibles.list").catch(() => []);
    let saved: string | null = null;
    try {
      saved = localStorage.getItem(BIBLE_KEY);
    } catch {
      // Not remembered.
    }
    bibleId = bibles.find((bible) => bible.id === saved)?.id ?? bibles[0]?.id ?? null;
  });

  // Understand the reference as it is typed.
  $effect(() => {
    const text = query.trim();
    previewed = null;
    if (!text) {
      feedback = null;
      return;
    }
    const timer = setTimeout(async () => {
      feedback = await remote
        .call<ReferenceFeedback>("scripture.parse", { text })
        .catch(() => null);
    }, 150);
    return () => clearTimeout(timer);
  });

  function chooseBible(id: string) {
    bibleId = id;
    try {
      localStorage.setItem(BIBLE_KEY, id);
    } catch {
      // Not remembered.
    }
  }

  async function enter() {
    const passage = feedback?.passage;
    if (!passage || !bibleId) return;
    if (previewed) {
      await remote.dispatch({ type: "goLive" });
      previewed = null;
      return;
    }
    await remote
      .call("scripture.preview", {
        passage,
        bibleIds: [bibleId],
        options: { maxVerses: 2, maxChars: 300, verseNumbers: null },
      })
      .then(() => (previewed = passage))
      .catch((error: Error) => remote.showError(error.message));
  }
</script>

<section class="scripture">
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void enter();
    }}
  >
    <input
      bind:value={query}
      type="search"
      enterkeyhint="go"
      autocapitalize="off"
      autocomplete="off"
      placeholder={t("remote.referencePlaceholder")}
      aria-label={t("scripture.search")}
    >
    <select
      value={bibleId}
      aria-label={t("scripture.translation")}
      onchange={(e) => chooseBible(e.currentTarget.value)}
    >
      {#each bibles as bible (bible.id)}
        <option value={bible.id}>{bible.abbreviation}</option>
      {/each}
    </select>
  </form>
  <p class="feedback" class:error={feedback?.error}>
    {#if feedback?.error}
      {feedback.error}
    {:else if feedback}
      {feedback.display}
    {:else}
      {t("remote.referenceHint")}
    {/if}
  </p>
  <button type="button" class="action" disabled={!feedback?.passage} onclick={() => void enter()}>
    {previewed ? t("pane.goLive") : t("common.showInPreview")}
  </button>
</section>

<PreviewCard />

<style>
  .scripture {
    padding: 12px;
  }

  form {
    display: flex;
    gap: 8px;
  }

  input,
  select {
    min-height: 52px;
    padding: 0 12px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  input {
    flex: 1;
    min-width: 0;
    font-size: 20px;
  }

  .feedback {
    min-height: 1.5em;
    margin: 10px 2px;
    color: var(--sw-text-muted);
  }

  .feedback.error {
    color: var(--sw-warning, #f59f00);
  }

  .action {
    width: 100%;
    min-height: 56px;
    border: none;
    border-radius: 12px;
    background: var(--sw-accent);
    color: #fff;
    font-size: 17px;
    font-weight: 600;
  }

  .action:disabled {
    opacity: 0.4;
  }
</style>
