<!--
  @component
  Import from Other Software, in three steps:

  1. **Sources**: libraries found on this computer (EasyWorship, OpenLP,
     ProPresenter), plus any folder the operator picks.
  2. **Review**: every song and Bible found, and EasyWorship's pictures,
     videos and sound. Songs and media already in the library are marked
     and left unticked; Bible names can be corrected. Media files that are
     not where EasyWorship keeps them can be looked for in another folder.
  3. **Done**: what was added.
-->
<script lang="ts">
  import type {
    FoundSource,
    ImportPreview,
    ImportSummary,
    PreviewBible,
    PreviewMedia,
  } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { commitImport, locateImportMedia, previewImport, scanForImports } from "../lib/api.ts";
  import { listOf, t } from "../lib/i18n/i18n.svelte.ts";
  import { importer } from "../lib/importer.svelte.ts";
  import { library } from "../lib/library.svelte.ts";

  type Step = "sources" | "review" | "done";

  let step: Step = $state("sources");
  let busy: string | null = $state(null);
  let error: string | null = $state(null);

  let sources: FoundSource[] = $state([]);
  // SvelteSet: a Set whose additions and removals update the page.
  const chosenSources = new SvelteSet<string>();

  let preview: ImportPreview | null = $state(null);
  let chosenSongs = $state(new SvelteSet<number>());
  let bibles: (PreviewBible & { chosen: boolean })[] = $state([]);
  let media: PreviewMedia[] = $state.raw([]);
  /** Whether to copy the media that was found. */
  let importMedia = $state(true);
  let summary: ImportSummary | null = $state(null);

  const songCount = $derived(chosenSongs.size);
  const bibleCount = $derived(bibles.filter((bible) => bible.chosen).length);
  /** Media that can be copied: found, a format SimpleWorship shows, and new. */
  const importable = $derived(media.filter((item) => item.path && item.kind && !item.duplicate));
  const missing = $derived(media.filter((item) => !item.path));
  const unsupported = $derived(media.filter((item) => item.path && !item.kind));
  const mediaCount = $derived(importMedia ? importable.length : 0);
  const countOf = (kind: string) => importable.filter((item) => item.kind === kind).length;

  onMount(async () => {
    if (importer.initialSources) {
      addSources(importer.initialSources);
    } else {
      await run(t("import.looking"), async () => {
        addSources(await scanForImports());
      });
    }
  });

  function key(source: FoundSource): string {
    return `${source.kind}:${source.path}`;
  }

  /** Adds newly found sources, ticked. */
  function addSources(found: FoundSource[]) {
    for (const source of found) {
      if (!sources.some((known) => key(known) === key(source))) {
        sources.push(source);
        chosenSources.add(key(source));
      }
    }
  }

  async function chooseFolder() {
    const folder = await open({ directory: true, multiple: false });
    if (typeof folder !== "string") return;
    await run(t("import.searchingFolder"), async () => {
      const found = await scanForImports(folder);
      if (found.length === 0) error = t("import.nothingInFolder");
      addSources(found);
    });
  }

  async function review() {
    const selected = sources.filter((source) => chosenSources.has(key(source)));
    await run(t("import.reading"), async () => {
      const result = await previewImport(selected);
      preview = result;
      chosenSongs = new SvelteSet(
        result.songs.flatMap((entry, index) => (entry.duplicate ? [] : [index])),
      );
      bibles = result.bibles.map((bible) => ({ ...bible, chosen: !bible.installed }));
      media = result.media;
      step = "review";
    });
  }

  async function commit() {
    if (!preview) return;
    const songs = preview.songs
      .filter((_, index) => chosenSongs.has(index))
      .map((entry) => entry.song);
    const choices = bibles
      .filter((bible) => bible.chosen)
      .map(({ path, name, abbreviation }) => ({ path, name, abbreviation }));
    const files = importMedia
      ? importable.map((item) => ({ path: item.path ?? "", title: item.title }))
      : [];
    const what = listOf([
      t("count.songs", { count: songs.length }),
      t("count.bibles", { count: choices.length }),
      ...(files.length > 0 ? [t("count.media", { count: files.length })] : []),
    ]);
    await run(t("import.importing", { what }), async () => {
      summary = await commitImport(songs, choices, files);
      library.changed();
      step = "done";
    });
  }

  /** "Imported 12 songs, 1 Bible and 40 pictures and videos." */
  function doneMessage(result: ImportSummary): string {
    const parts = [t("count.songs", { count: result.songs })];
    if (result.bibles > 0) parts.push(t("count.bibles", { count: result.bibles }));
    if (result.media > 0) parts.push(t("count.media", { count: result.media }));
    return t("import.done", { what: listOf(parts) });
  }

  /** Looks for the media files that were not found, in a folder the operator picks. */
  async function locateMedia() {
    const folder = await open({ directory: true, multiple: false });
    if (typeof folder !== "string") return;
    await run(t("import.lookingFiles"), async () => {
      const before = missing.length;
      media = await locateImportMedia(media, folder);
      const found = before - media.filter((item) => !item.path).length;
      if (found === 0) error = t("import.noneInFolder");
    });
  }

  /** Runs a step with a busy message, showing any error. */
  async function run(message: string, work: () => Promise<void>) {
    busy = message;
    error = null;
    try {
      await work();
    } catch (failure) {
      error = String(failure);
    } finally {
      busy = null;
    }
  }

  function toggle<T>(set: Set<T>, value: T, on: boolean) {
    if (on) set.add(value);
    else set.delete(value);
  }

  function setAllSongs(on: boolean, onlyNew = false) {
    chosenSongs = new SvelteSet(
      (preview?.songs ?? []).flatMap((entry, index) =>
        on && !(onlyNew && entry.duplicate) ? [index] : [],
      ),
    );
  }

  const kindLabel: Record<FoundSource["kind"], string> = $derived({
    easyWorshipSongs: t("import.songLibrary"),
    easyWorshipBible: t("import.bible"),
    openLpSongs: t("import.songLibrary"),
    songFiles: t("import.songFiles"),
  });
</script>

<Dialog title={t("import.title")} onclose={() => importer.close()}>
  {#if step === "sources"}
    <p class="intro">{t("import.intro")}</p>
    {#if sources.length > 0}
      <ul class="sources">
        {#each sources as source (key(source))}
          <li>
            <label>
              <input
                type="checkbox"
                checked={chosenSources.has(key(source))}
                onchange={(e) => toggle(chosenSources, key(source), e.currentTarget.checked)}
              >
              <span class="kind">{kindLabel[source.kind]}</span>
              <span class="label">{source.label}</span>
              <span class="path" title={source.path}>{source.path}</span>
            </label>
          </li>
        {/each}
      </ul>
    {:else if !busy}
      <p class="hint">{t("import.nothingFound")}</p>
    {/if}
  {:else if step === "review" && preview}
    <section>
      <header class="section-head">
        <h4>{t("import.songsChosen", { chosen: songCount, total: preview.songs.length })}</h4>
        <Button onclick={() => setAllSongs(true, true)}>{t("import.newOnly")}</Button>
        <Button onclick={() => setAllSongs(true)}>{t("common.all")}</Button>
        <Button onclick={() => setAllSongs(false)}>{t("common.none")}</Button>
      </header>
      <div class="table">
        {#each preview.songs as entry, index (index)}
          <label class="row">
            <input
              type="checkbox"
              checked={chosenSongs.has(index)}
              onchange={(e) => toggle(chosenSongs, index, e.currentTarget.checked)}
            >
            <span class="title">{entry.song.title}</span>
            <span class="author">{entry.song.author}</span>
            {#if entry.duplicate}
              <span class="badge">{t("import.inLibrary")}</span>
            {/if}
          </label>
        {:else}
          <p class="hint">{t("import.noSongs")}</p>
        {/each}
      </div>
    </section>

    {#if bibles.length > 0}
      <section>
        <h4>{t("import.biblesChosen", { chosen: bibleCount, total: bibles.length })}</h4>
        {#each bibles as bible (bible.path)}
          <div class="bible">
            <input
              type="checkbox"
              bind:checked={bible.chosen}
              aria-label={t("import.importBible", { name: bible.name })}
            >
            <input
              class="abbreviation"
              bind:value={bible.abbreviation}
              aria-label={t("bibles.shortName")}
            >
            <input class="bible-name" bind:value={bible.name} aria-label={t("common.name")}>
            <span class="author">
              {t("import.verses", { count: bible.verseCount.toLocaleString() })}
            </span>
            {#if bible.installed}
              <span class="badge">{t("import.installed")}</span>
            {/if}
          </div>
        {/each}
        <p class="hint">{t("import.bibleHint")}</p>
      </section>
    {/if}

    {#if media.length > 0}
      <section>
        <h4>{t("import.mediaTitle")}</h4>
        <label class="row">
          <input type="checkbox" bind:checked={importMedia} disabled={importable.length === 0}>
          <span class="title">
            {t("import.copyMedia", {
              what: listOf([
                t("count.pictures", { count: countOf("image") }),
                t("count.videos", { count: countOf("video") }),
                ...(countOf("audio") > 0
                  ? [t("count.soundFiles", { count: countOf("audio") })]
                  : []),
              ]),
            })}
          </span>
        </label>
        {#if media.some((item) => item.duplicate)}
          <p class="hint">
            {t("import.mediaDuplicates", { count: media.filter((item) => item.duplicate).length })}
          </p>
        {/if}
        {#if missing.length > 0}
          <div class="missing">
            <span>
              {t("import.mediaMissing", { missing: missing.length, total: media.length })}
              <span class="searched">
                {missing[0]?.searched
                  ? t("import.searchedIn", { folder: missing[0].searched })
                  : t("import.noResources")}
              </span>
            </span>
            <Button disabled={busy !== null} onclick={locateMedia}
              >{t("import.lookElsewhere")}</Button
            >
          </div>
        {/if}
        {#if unsupported.length > 0}
          <details>
            <summary>{t("import.unsupported", { count: unsupported.length })}</summary>
            <p class="hint">{t("import.unsupportedHint")}</p>
            <ul class="problems">
              {#each unsupported as item (item.dataFolder + item.fileName)}
                <li>{item.title} ({item.fileName})</li>
              {/each}
            </ul>
          </details>
        {/if}
      </section>
    {/if}

    {#if preview.problems.length > 0}
      <details>
        <summary>{t("import.problems", { count: preview.problems.length })}</summary>
        <ul class="problems">
          {#each preview.problems as problem, index (index)}
            <li><strong>{problem.source}</strong>: {problem.message}</li>
          {/each}
        </ul>
      </details>
    {/if}
  {:else if step === "done" && summary}
    <p class="done">{doneMessage(summary)}</p>
    {#each summary.problems as problem, index (index)}
      <p class="error">{problem.source}: {problem.message}</p>
    {/each}
  {/if}

  {#snippet actions()}
    {#if busy}
      <span class="status">{busy}</span>
    {:else if error}
      <span class="error" role="alert">{error}</span>
    {/if}

    {#if step === "sources"}
      <Button disabled={busy !== null} onclick={chooseFolder}>{t("import.chooseFolder")}</Button>
      <Button onclick={() => importer.close()}>{t("common.cancel")}</Button>
      <Button
        variant="primary"
        disabled={busy !== null || chosenSources.size === 0}
        onclick={review}
      >
        {t("common.continue")}
      </Button>
    {:else if step === "review"}
      <Button disabled={busy !== null} onclick={() => (step = "sources")}
        >{t("common.back")}</Button
      >
      <Button
        variant="primary"
        disabled={busy !== null || songCount + bibleCount + mediaCount === 0}
        onclick={commit}
      >
        {t("import.button", {
          what: listOf([
            t("count.songs", { count: songCount }),
            ...(bibleCount > 0 ? [t("count.bibles", { count: bibleCount })] : []),
            ...(mediaCount > 0 ? [t("count.media", { count: mediaCount })] : []),
          ]),
        })}
      </Button>
    {:else}
      <Button variant="primary" onclick={() => importer.close()}>{t("common.done")}</Button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .intro,
  .hint {
    margin: 0 0 var(--sw-space-3);
    color: var(--sw-text-muted);
    line-height: 1.5;
  }

  .hint {
    color: var(--sw-text-faint);
  }

  .sources {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
  }

  .sources label,
  .row,
  .bible {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .kind,
  .badge {
    padding: 0 6px;
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
    white-space: nowrap;
  }

  .label {
    font-weight: 600;
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  section {
    margin-bottom: var(--sw-space-4);
  }

  .section-head {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
  }

  h4 {
    flex: 1;
    margin: 0 0 var(--sw-space-2);
    font-size: var(--sw-font-size);
  }

  .table {
    max-height: 40vh;
    overflow-y: auto;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
  }

  .row {
    padding: 3px var(--sw-space-2);
  }

  .row:nth-child(even) {
    background: var(--sw-tint);
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .author {
    color: var(--sw-text-muted);
    white-space: nowrap;
  }

  .bible {
    margin-bottom: var(--sw-space-1);
  }

  .abbreviation {
    width: 80px;
  }

  .bible-name {
    flex: 1;
  }

  .problems {
    margin: var(--sw-space-2) 0 0;
    padding-left: var(--sw-space-4);
    color: var(--sw-text-muted);
    max-height: 20vh;
    overflow-y: auto;
  }

  .done {
    font-size: 1.1em;
  }

  .searched {
    display: block;
    margin-top: 2px;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
    word-break: break-all;
  }

  .missing {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    margin: var(--sw-space-2) 0;
    padding: var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: rgba(245, 159, 0, 0.12);
    color: var(--sw-text-muted);
  }

  .status {
    margin-right: auto;
    color: var(--sw-text-muted);
  }

  .error {
    margin-right: auto;
    color: var(--sw-live);
  }
</style>
