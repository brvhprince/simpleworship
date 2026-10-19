<!--
  @component
  Import from Other Software, in three steps:

  1. **Sources**: libraries found on this computer (EasyWorship, OpenLP,
     ProPresenter), plus any folder the operator picks.
  2. **Review**: every song and Bible found. Songs already in the library
     are marked and left unticked; Bible names can be corrected.
  3. **Done**: what was added.
-->
<script lang="ts">
  import type {
    FoundSource,
    ImportPreview,
    ImportSummary,
    PreviewBible,
  } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { commitImport, previewImport, scanForImports } from "../lib/api.ts";
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
  let summary: ImportSummary | null = $state(null);

  const songCount = $derived(chosenSongs.size);
  const bibleCount = $derived(bibles.filter((bible) => bible.chosen).length);

  onMount(async () => {
    if (importer.initialSources) {
      addSources(importer.initialSources);
    } else {
      await run("Looking for other worship software…", async () => {
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
    await run("Searching the folder…", async () => {
      const found = await scanForImports(folder);
      if (found.length === 0) error = "Nothing SimpleWorship can import was found in that folder.";
      addSources(found);
    });
  }

  async function review() {
    const selected = sources.filter((source) => chosenSources.has(key(source)));
    await run("Reading songs and Bibles…", async () => {
      const result = await previewImport(selected);
      preview = result;
      chosenSongs = new SvelteSet(
        result.songs.flatMap((entry, index) => (entry.duplicate ? [] : [index])),
      );
      bibles = result.bibles.map((bible) => ({ ...bible, chosen: !bible.installed }));
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
    await run(`Importing ${songs.length} songs and ${choices.length} Bibles…`, async () => {
      summary = await commitImport(songs, choices);
      library.changed();
      step = "done";
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

  const kindLabel: Record<FoundSource["kind"], string> = {
    easyWorshipSongs: "Song library",
    easyWorshipBible: "Bible",
    openLpSongs: "Song library",
    songFiles: "Song files",
  };
</script>

<Dialog title="Import from Other Software" onclose={() => importer.close()}>
  {#if step === "sources"}
    <p class="intro">
      SimpleWorship can import songs from EasyWorship 6/7, OpenLP, ProPresenter 6, OpenLyrics,
      ChordPro and CCLI SongSelect files, and Bibles from EasyWorship. The other program's files are
      only read, never changed.
    </p>
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
      <p class="hint">
        Nothing was found in the usual places. Choose a folder: for example a copy of EasyWorship's
        folders from a Windows computer, or a folder of song files.
      </p>
    {/if}
  {:else if step === "review" && preview}
    <section>
      <header class="section-head">
        <h4>Songs · {songCount} of {preview.songs.length} chosen</h4>
        <Button onclick={() => setAllSongs(true, true)}>New Only</Button>
        <Button onclick={() => setAllSongs(true)}>All</Button>
        <Button onclick={() => setAllSongs(false)}>None</Button>
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
              <span class="badge">Already in library</span>
            {/if}
          </label>
        {:else}
          <p class="hint">No songs.</p>
        {/each}
      </div>
    </section>

    {#if bibles.length > 0}
      <section>
        <h4>Bibles · {bibleCount} of {bibles.length} chosen</h4>
        {#each bibles as bible (bible.path)}
          <div class="bible">
            <input type="checkbox" bind:checked={bible.chosen} aria-label="Import {bible.name}">
            <input class="abbreviation" bind:value={bible.abbreviation} aria-label="Short name">
            <input class="bible-name" bind:value={bible.name} aria-label="Name">
            <span class="author">{bible.verseCount.toLocaleString()} verses</span>
            {#if bible.installed}
              <span class="badge">Installed</span>
            {/if}
          </div>
        {/each}
        <p class="hint">
          Check the short names: a Bible copied from another may still carry the other's name. Only
          import Bibles you are licensed to use.
        </p>
      </section>
    {/if}

    {#if preview.problems.length > 0}
      <details>
        <summary>{preview.problems.length} could not be imported</summary>
        <ul class="problems">
          {#each preview.problems as problem, index (index)}
            <li><strong>{problem.source}</strong>: {problem.message}</li>
          {/each}
        </ul>
      </details>
    {/if}
  {:else if step === "done" && summary}
    <p class="done">
      Imported {summary.songs} {summary.songs === 1 ? "song" : "songs"}
      {#if summary.bibles > 0}
        and {summary.bibles} {summary.bibles === 1 ? "Bible" : "Bibles"}
      {/if}
      .
    </p>
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
      <Button disabled={busy !== null} onclick={chooseFolder}>Choose Folder…</Button>
      <Button onclick={() => importer.close()}>Cancel</Button>
      <Button
        variant="primary"
        disabled={busy !== null || chosenSources.size === 0}
        onclick={review}
      >
        Continue
      </Button>
    {:else if step === "review"}
      <Button disabled={busy !== null} onclick={() => (step = "sources")}>Back</Button>
      <Button
        variant="primary"
        disabled={busy !== null || songCount + bibleCount === 0}
        onclick={commit}
      >
        Import {songCount} {songCount === 1 ? "Song" : "Songs"}{bibleCount > 0
          ? ` and ${bibleCount} ${bibleCount === 1 ? "Bible" : "Bibles"}`
          : ""}
      </Button>
    {:else}
      <Button variant="primary" onclick={() => importer.close()}>Done</Button>
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
    background: rgba(255, 255, 255, 0.02);
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

  .status {
    margin-right: auto;
    color: var(--sw-text-muted);
  }

  .error {
    margin-right: auto;
    color: var(--sw-live);
  }
</style>
