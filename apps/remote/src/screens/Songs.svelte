<!--
  @component
  The song library: search by title, author or words, tap to preview, then
  Go Live.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import type { SongSummary } from "@simpleworship/protocol";
  import { remote } from "../lib/remote.svelte.ts";
  import PreviewCard from "./PreviewCard.svelte";

  let query = $state("");
  let results: SongSummary[] = $state([]);

  $effect(() => {
    const text = query;
    const timer = setTimeout(async () => {
      results = await remote.call<SongSummary[]>("songs.search", { query: text }).catch(() => []);
    }, 200);
    return () => clearTimeout(timer);
  });

  function preview(song: SongSummary) {
    remote
      .call("songs.preview", { id: song.id })
      .catch((error: Error) => remote.showError(error.message));
  }
</script>

<section class="songs">
  <input
    bind:value={query}
    type="search"
    autocomplete="off"
    placeholder={t("songs.search")}
    aria-label={t("songs.searchLabel")}
  >
  <ul>
    {#each results.slice(0, 100) as song (song.id)}
      <li>
        <button type="button" onclick={() => preview(song)}>
          <span class="title">{song.title}</span>
          <span class="author">{song.author}</span>
        </button>
      </li>
    {:else}
      <li class="empty">{query ? t("songs.noMatch") : t("songs.empty")}</li>
    {/each}
  </ul>
</section>

<PreviewCard />

<style>
  .songs {
    padding: 12px;
  }

  input {
    width: 100%;
    box-sizing: border-box;
    min-height: 50px;
    padding: 0 12px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  ul {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
  }

  li button {
    width: 100%;
    min-height: 54px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 6px 12px;
    margin-bottom: 4px;
    border: none;
    border-radius: 8px;
    background: var(--sw-surface);
    color: var(--sw-text);
    text-align: left;
  }

  .author {
    color: var(--sw-text-muted);
    font-size: 13px;
  }

  .empty {
    padding: 20px;
    color: var(--sw-text-faint);
    text-align: center;
  }
</style>
