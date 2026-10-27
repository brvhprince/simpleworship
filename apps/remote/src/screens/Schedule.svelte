<!--
  @component
  The order of service. Tap an item to preview it, then Go Live. For the
  worship leader (`songsOnly`), only the songs, each sent live in one tap.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import type { ScheduleItem } from "@simpleworship/protocol";
  import { remote } from "../lib/remote.svelte.ts";
  import PreviewCard from "./PreviewCard.svelte";

  interface Props {
    songsOnly?: boolean;
  }

  let { songsOnly = false }: Props = $props();

  const items = $derived(
    remote.schedule.items.filter((item) => !songsOnly || item.content.type === "song"),
  );
  const liveId = $derived(remote.live?.scheduleItem ?? null);
  const previewId = $derived(remote.preview?.scheduleItem ?? null);

  function title(item: ScheduleItem): string {
    switch (item.content.type) {
      case "song":
        return item.content.song.title;
      case "text":
      case "countdown":
        return item.content.title;
      case "scripture":
        return item.content.reference;
      case "media":
        return item.content.media.name;
      case "camera":
        return item.content.camera.label;
      case "slides":
      case "playlist":
        return item.content.name;
      case "deck":
        return item.content.deck.name;
      case "sermon":
        return item.content.title;
    }
  }

  function tap(item: ScheduleItem) {
    if (songsOnly) void remote.dispatch({ type: "showScheduleItem", id: item.id });
    else void remote.dispatch({ type: "previewScheduleItem", id: item.id });
  }
</script>

<section class="schedule">
  <ul>
    {#each items as item, index (item.id)}
      <li>
        <button
          type="button"
          class:live={item.id === liveId}
          class:preview={item.id === previewId}
          onclick={() => tap(item)}
        >
          <span class="number">{index + 1}</span>
          <span class="title">{title(item)}</span>
          {#if item.id === liveId}
            <span class="tag">{t("pane.live")}</span>
          {:else if songsOnly}
            <span class="go">{t("pane.goLive")}</span>
          {/if}
        </button>
      </li>
    {:else}
      <li class="empty">{songsOnly ? t("remote.noSongsInSchedule") : t("remote.emptySchedule")}</li>
    {/each}
  </ul>
</section>

{#if !songsOnly}
  <PreviewCard />
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 8px;
  }

  li button {
    width: 100%;
    min-height: 56px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px;
    margin-bottom: 6px;
    border: 1px solid var(--sw-border);
    border-left: 4px solid transparent;
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
    text-align: left;
  }

  li button.preview {
    border-left-color: var(--sw-preview);
  }

  li button.live {
    border-left-color: var(--sw-live);
    background: var(--sw-live-soft);
  }

  .number {
    width: 1.5em;
    color: var(--sw-text-faint);
  }

  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag,
  .go {
    font-size: 13px;
    font-weight: 600;
  }

  .tag {
    color: var(--sw-live);
  }

  .go {
    color: var(--sw-accent);
  }

  .empty {
    padding: 24px;
    color: var(--sw-text-faint);
    text-align: center;
  }
</style>
