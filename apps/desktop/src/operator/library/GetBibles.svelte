<!--
  @component
  Get More Bibles: eBible.org's translations that may be shared freely, to
  search and download into the library (see `src-tauri/src/bible_catalog.rs`).

  Bibles in the interface's language come first. Whole Bibles are shown
  unless the operator asks for New Testaments and portions too. Without
  internet the list kept from before is shown, marked as possibly out of
  date. A download shows its progress, then becomes "Installed".
-->
<script lang="ts">
  import type {
    BibleCatalog,
    BibleDownload,
    CatalogBible,
    Coverage,
  } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { bibleCatalog, installCatalogBible } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { i18n, type MessageKey, t } from "../../lib/i18n/i18n.svelte.ts";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  /** Must match `DOWNLOAD_EVENT` in `src-tauri/src/bible_catalog.rs`. */
  const DOWNLOAD_EVENT = "bibles://download";

  /** Each interface language's ISO 639-3 code, to suggest its Bibles first. */
  const SUGGESTED: Record<string, string> = { en: "eng", fr: "fra", tw: "twi" };

  const COVERAGE: Record<Coverage, MessageKey> = {
    bible: "catalog.coverageBible",
    newTestament: "catalog.coverageNewTestament",
    oldTestament: "catalog.coverageOldTestament",
    portions: "catalog.coveragePortions",
  };

  let catalog: BibleCatalog | null = $state.raw(null);
  let loading = $state(true);
  let problem: string | null = $state(null);
  let query = $state("");
  let wholeOnly = $state(true);
  /** Downloads in progress: progress from 0 to 1, by id. */
  let downloads: Record<string, number> = $state({});

  async function load(refresh: boolean) {
    loading = true;
    problem = null;
    try {
      catalog = await bibleCatalog(refresh);
    } catch (error) {
      problem = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load(false);
    const stop = listen<BibleDownload>(DOWNLOAD_EVENT, ({ payload }) => {
      if (payload.id in downloads) downloads[payload.id] = payload.progress;
    });
    return () => void stop.then((unlisten) => unlisten());
  });

  /** Everything a search can match: names, title, languages and id. */
  function searchable(bible: CatalogBible): string {
    return [bible.name, bible.title, bible.language, bible.languageEnglish, bible.id]
      .join(" ")
      .toLowerCase();
  }

  /** What the search and "whole Bibles only" let through, suggested ones first. */
  const shown = $derived.by(() => {
    if (!catalog) return [];
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    const suggested = SUGGESTED[i18n.language];
    const matching = catalog.bibles.filter(
      (bible) =>
        (!wholeOnly || bible.coverage === "bible") &&
        words.every((word) => searchable(bible).includes(word)),
    );
    // A stable sort keeps the catalogue's order (by language, then name).
    return [...matching].sort(
      (a, b) => Number(b.languageCode === suggested) - Number(a.languageCode === suggested),
    );
  });

  async function install(bible: CatalogBible) {
    downloads[bible.id] = 0;
    try {
      await installCatalogBible(bible.id);
      if (catalog) {
        catalog = {
          ...catalog,
          bibles: catalog.bibles.map((other) =>
            other.id === bible.id ? { ...other, installed: true } : other,
          ),
        };
      }
    } catch (error) {
      engine.showError(error);
    } finally {
      delete downloads[bible.id];
    }
  }

  /** "Akuapem Twi · Twi", or just "English" when both names are the same. */
  function languageOf(bible: CatalogBible): string {
    const english = bible.languageEnglish;
    return english && english !== bible.language
      ? `${bible.language} · ${english}`
      : bible.language;
  }
</script>

<Dialog title={t("catalog.title")} {onclose}>
  <div class="toolbar">
    <input
      type="search"
      class="search"
      bind:value={query}
      placeholder={t("catalog.search")}
      aria-label={t("catalog.search")}
    >
    <label class="option">
      <input type="checkbox" bind:checked={wholeOnly}>
      {t("catalog.wholeOnly")}
    </label>
    <span class="spacer"></span>
    <Button disabled={loading} onclick={() => load(true)}>{t("catalog.refresh")}</Button>
  </div>

  {#if catalog?.offline}
    <p class="notice">{t("catalog.offline")}</p>
  {/if}

  {#if !catalog}
    <p class="hint">{loading ? t("catalog.loading") : problem}</p>
  {:else}
    <ul class="list" aria-label={t("catalog.title")}>
      {#each shown as bible (bible.id)}
        {@const progress = downloads[bible.id]}
        <li>
          <div class="about" dir={bible.rightToLeft ? "rtl" : null}>
            <div class="name">{bible.name}</div>
            <div class="meta">
              {languageOf(bible)}
              · {t(COVERAGE[bible.coverage])} · {bible.abbreviation}
            </div>
            {#if bible.copyright}
              <div class="copyright" title={bible.copyright}>{bible.copyright}</div>
            {/if}
          </div>
          {#if bible.installed}
            <span class="installed">✓ {t("catalog.installed")}</span>
          {:else if progress !== undefined}
            <span class="progress">
              {progress >= 1
                ? t("catalog.adding")
                : t("catalog.downloading", { percent: Math.round(progress * 100) })}
            </span>
          {:else}
            <Button onclick={() => install(bible)}>{t("catalog.install")}</Button>
          {/if}
        </li>
      {:else}
        <li class="hint">{t("catalog.nothing")}</li>
      {/each}
    </ul>
  {/if}

  <p class="licence">{t("catalog.licence")}</p>

  {#snippet actions()}
    <Button variant="primary" onclick={onclose}>{t("common.done")}</Button>
  {/snippet}
</Dialog>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--sw-space-3);
    margin-bottom: var(--sw-space-3);
  }

  .search {
    width: 280px;
  }

  .option {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  .spacer {
    flex: 1;
  }

  .notice {
    margin: 0 0 var(--sw-space-3);
    color: var(--sw-text-muted);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 55vh;
    overflow-y: auto;
    border-top: 1px solid var(--sw-border);
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--sw-space-3);
    padding: var(--sw-space-2) var(--sw-space-1);
    border-bottom: 1px solid var(--sw-border);
  }

  .about {
    flex: 1;
    min-width: 0;
  }

  .name {
    font-weight: 600;
  }

  .meta,
  .copyright {
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
  }

  .copyright {
    color: var(--sw-text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .installed {
    color: var(--sw-text-muted);
    white-space: nowrap;
  }

  .progress {
    color: var(--sw-text-muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .hint {
    color: var(--sw-text-faint);
  }

  .licence {
    margin: var(--sw-space-3) 0 0;
    color: var(--sw-text-muted);
    line-height: 1.5;
  }
</style>
