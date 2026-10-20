<!--
  @component
  Sermon notes to slides, with the assistant: paste the preacher's notes,
  and it lists the points (as text slides) and every passage they refer
  to, in order. Untick anything not wanted, then add them all to the
  schedule. Passages use the translations chosen in the Scripture tab.
-->
<script lang="ts">
  import type { Passage, ScriptureOptions, SermonItem } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { addScriptureToSchedule, addTextToSchedule, parseReference } from "../../lib/api.ts";
  import { assistant } from "../../lib/assistant.svelte.ts";
  import { bibles } from "../../lib/bibles.svelte.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  /** One thing to add, ticked or not. A passage that could not be read
   *  has no `passage` and cannot be added. */
  interface Row {
    item: SermonItem;
    passage: Passage | null;
    include: boolean;
  }

  let notes = $state("");
  let rows: Row[] = $state([]);
  let adding = $state(false);
  let problem: string | null = $state(null);

  /** The Scripture tab's choices (remembered on this computer). */
  function scriptureChoices(): { bibleIds: string[]; options: ScriptureOptions } {
    let saved: { primaryId?: string | null; secondaryId?: string | null; maxVerses?: number } = {};
    try {
      saved = JSON.parse(localStorage.getItem("simpleworship.scripture") ?? "{}");
    } catch {
      // Nothing remembered: the first Bible, two verses a slide.
    }
    const ids = bibles.list.map((bible) => bible.id);
    const primary = saved.primaryId && ids.includes(saved.primaryId) ? saved.primaryId : ids[0];
    const secondary =
      saved.secondaryId && ids.includes(saved.secondaryId) ? saved.secondaryId : null;
    return {
      bibleIds: [primary, secondary].filter((id): id is string => !!id),
      options: { maxVerses: saved.maxVerses ?? 2, maxChars: 300, verseNumbers: null },
    };
  }

  async function makeSlides() {
    problem = null;
    try {
      const items = await assistant.sermonSlides(notes);
      const made: Row[] = [];
      for (const item of items) {
        if (item.kind === "scripture") {
          const passage = (await parseReference(item.reference)).passage ?? null;
          made.push({ item, passage, include: passage !== null });
        } else {
          made.push({ item, passage: null, include: true });
        }
      }
      rows = made;
    } catch (error) {
      problem = String(error);
    }
  }

  async function addAll() {
    adding = true;
    problem = null;
    const { bibleIds, options } = scriptureChoices();
    try {
      for (const row of rows) {
        if (!row.include) continue;
        if (row.item.kind === "point") {
          await addTextToSchedule(row.item.title, row.item.text);
        } else if (row.passage) {
          await addScriptureToSchedule(row.passage, bibleIds, options);
        }
      }
      onclose();
    } catch (error) {
      engine.showError(error);
    } finally {
      adding = false;
    }
  }
</script>

<Dialog title={t("sermon.title")} {onclose}>
  <div class="sermon">
    <label class="notes">
      {t("sermon.notes")}
      <textarea bind:value={notes} placeholder={t("sermon.placeholder")}></textarea>
    </label>

    <div class="result">
      {#if rows.length > 0}
        <ul>
          {#each rows as row, index (index)}
            <li class:off={!row.include}>
              <!-- biome-ignore lint/a11y/noLabelWithoutControl: the checkbox and the words are inside the label (in Svelte blocks Biome does not read) -->
              <label>
                <input
                  type="checkbox"
                  bind:checked={row.include}
                  disabled={row.item.kind === "scripture" && !row.passage}
                >
                {#if row.item.kind === "point"}
                  <span class="kind">{t("sermon.point")}</span>
                  <span>
                    <strong>{row.item.title}</strong>
                    <span class="text">{row.item.text}</span>
                  </span>
                {:else}
                  <span class="kind">{t("sermon.scripture")}</span>
                  <strong>{row.item.reference}</strong>
                  {#if !row.passage}
                    <span class="problem">{t("sermon.notFound")}</span>
                  {/if}
                {/if}
              </label>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="hint">{t("sermon.hint")}</p>
      {/if}
    </div>
  </div>

  {#snippet actions()}
    {#if problem}
      <span class="problem" role="alert">{problem}</span>
    {/if}
    <Button onclick={onclose}>{t("common.cancel")}</Button>
    <Button disabled={!notes.trim() || assistant.busy} onclick={makeSlides}>
      {assistant.busy ? t("assistant.thinking") : t("sermon.make")}
    </Button>
    <Button
      variant="primary"
      disabled={adding || !rows.some((row) => row.include)}
      onclick={addAll}
    >
      {t("sermon.add", { count: rows.filter((row) => row.include).length })}
    </Button>
  {/snippet}
</Dialog>

<style>
  .sermon {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sw-space-4);
    width: min(860px, 80vw);
    height: min(60vh, 560px);
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
  }

  textarea {
    flex: 1;
    resize: none;
  }

  .result {
    overflow-y: auto;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    padding: var(--sw-space-1) 0;
    border-bottom: 1px solid var(--sw-border);
  }

  li.off {
    opacity: 0.5;
  }

  label {
    display: flex;
    align-items: baseline;
    gap: var(--sw-space-2);
  }

  .kind {
    flex: none;
    width: 5.5em;
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
    text-transform: uppercase;
  }

  .text {
    display: block;
    color: var(--sw-text-muted);
    white-space: pre-line;
  }

  .hint {
    color: var(--sw-text-muted);
  }

  .problem {
    color: var(--sw-live);
  }
</style>
