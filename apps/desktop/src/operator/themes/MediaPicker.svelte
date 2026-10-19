<!--
  @component
  Chooses an image or video from the media library, shown as a row of
  thumbnails. Used by the theme designer and for per-item backgrounds.
-->
<script lang="ts">
  import type { MediaItem, MediaKind, MediaRef } from "@simpleworship/protocol";
  import { getMediaUrls } from "@simpleworship/renderer";
  import { media } from "../../lib/media.svelte.ts";

  interface Props {
    kind: MediaKind;
    value: MediaRef | null;
    onchange: (media: MediaItem) => void;
  }

  let { kind, value, onchange }: Props = $props();

  const urls = getMediaUrls();
  const choices = $derived(media.list.filter((item) => item.kind === kind));
</script>

<div class="picker" role="listbox" aria-label="Choose {kind === "video" ? "a video" : "an image"}">
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
    <p class="empty">
      No {kind === "video" ? "videos" : "images"} in the library yet. Import some in the Media tab.
    </p>
  {/each}
</div>

<style>
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
