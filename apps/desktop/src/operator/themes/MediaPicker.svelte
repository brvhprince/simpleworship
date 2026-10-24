<!--
  @component
  Chooses an image or video from the media library, shown as a row of
  thumbnails. Used by the theme designer and for per-item backgrounds.
-->
<script lang="ts">
  import type { MediaItem, MediaKind, MediaRef } from "@simpleworship/protocol";
  import { getMediaUrls } from "@simpleworship/renderer";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { media } from "../../lib/media.svelte.ts";

  interface Props {
    kind: MediaKind;
    value: MediaRef | null;
    onchange: (media: MediaItem) => void;
  }

  let { kind, value, onchange }: Props = $props();

  const urls = getMediaUrls();
  let search = $state("");
  const choices = $derived(
    media.list
      .filter((item) => item.kind === kind)
      .filter(
        (item) => !search.trim() || item.name.toLowerCase().includes(search.trim().toLowerCase()),
      ),
  );
</script>

<div class="media-picker-wrapper">
  <input
    type="search"
    class="picker-search"
    bind:value={search}
    placeholder={t("media.searchPlaceholder")}
    aria-label={t("common.search")}
  >
  <div
    class="picker"
    role="listbox"
    aria-label={kind === "video" ? t("picker.video") : t("picker.image")}
  >
    {#each choices as item (item.id)}
      <button
        type="button"
        role="option"
        aria-selected={item.id === value?.id}
        title={item.name}
        onclick={() => onchange(item)}
      >
        <img
          src="{urls.thumbnail({ id: item.id, file: item.file })}?v={media.thumbnailVersion}"
          alt={item.name}
        >
      </button>
    {:else}
      <p class="empty">{kind === "video" ? t("picker.noVideos") : t("picker.noImages")}</p>
    {/each}
  </div>
</div>

<style>
  .media-picker-wrapper {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    max-width: 100%;
  }

  .picker-search {
    width: 200px;
    padding: var(--sw-space-1);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    font-size: var(--sw-font-size-small);
  }

  .picker {
    display: flex;
    gap: var(--sw-space-1);
    overflow-x: auto;
    padding-bottom: var(--sw-space-1);
    max-width: 100%;
  }

  button {
    flex: none;
    width: 96px;
    aspect-ratio: 16 / 9;
    padding: 0;
    border: 2px solid transparent;
    border-radius: var(--sw-radius-small);
    background: #000;
    cursor: pointer;
    overflow: hidden;
  }

  button[aria-selected="true"] {
    border-color: var(--sw-accent);
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .empty {
    margin: 0;
    color: var(--sw-text-faint);
  }
</style>
