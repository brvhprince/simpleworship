<!--
  @component
  Picks pictures and videos from the media library, several at a time, in
  the order they are clicked. Import… brings new files into the library
  first. Used by the presentation editor's "Add Pictures".
-->
<script lang="ts">
  import { getMediaUrls } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { open } from "@tauri-apps/plugin-dialog";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { media } from "../../lib/media.svelte.ts";

  interface Props {
    /** Called with the chosen media ids, in the order they were picked. */
    onchoose: (ids: string[]) => void;
    onclose: () => void;
  }

  let { onchoose, onclose }: Props = $props();

  const urls = getMediaUrls();
  let chosen: string[] = $state([]);

  const choices = $derived(media.list.filter((item) => item.kind !== "audio"));

  function toggle(id: string) {
    chosen = chosen.includes(id) ? chosen.filter((other) => other !== id) : [...chosen, id];
  }

  async function importFiles() {
    const paths = await open({
      multiple: true,
      filters: [
        {
          name: t("pictures.filter"),
          extensions: [
            "png",
            "jpg",
            "jpeg",
            "gif",
            "webp",
            "bmp",
            "avif",
            "mp4",
            "m4v",
            "mov",
            "webm",
          ],
        },
      ],
    });
    if (paths && paths.length > 0) await media.import(paths);
  }
</script>

<Dialog title={t("pictures.title")} size="small" {onclose}>
  <div class="grid" role="listbox" aria-label={t("pictures.list")} aria-multiselectable="true">
    {#each choices as item (item.id)}
      {@const order = chosen.indexOf(item.id)}
      <button
        type="button"
        role="option"
        aria-selected={order !== -1}
        title={item.name}
        onclick={() => toggle(item.id)}
      >
        <img
          src="{urls.thumbnail({ id: item.id, file: item.file })}?v={media.thumbnailVersion}"
          alt={item.name}
        >
        {#if order !== -1}
          <span class="order">{order + 1}</span>
        {/if}
      </button>
    {:else}
      <p class="empty">{t("pictures.empty")}</p>
    {/each}
  </div>

  {#snippet actions()}
    <Button disabled={media.progress !== null} onclick={importFiles}>
      {media.progress ? t("media.copying", { name: media.progress.name }) : t("common.import")}
    </Button>
    <span class="spacer"></span>
    <Button onclick={onclose}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={chosen.length === 0} onclick={() => onchoose(chosen)}>
      {chosen.length ? t("pictures.add", { count: chosen.length }) : t("pictures.addNone")}
    </Button>
  {/snippet}
</Dialog>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: var(--sw-space-2);
    max-height: 50vh;
    overflow-y: auto;
  }

  button {
    position: relative;
    padding: 0;
    border: 2px solid transparent;
    border-radius: var(--sw-radius-small);
    background: #000;
    aspect-ratio: 16 / 9;
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

  .order {
    position: absolute;
    top: 4px;
    right: 4px;
    min-width: 20px;
    padding: 0 5px;
    border-radius: 10px;
    background: var(--sw-accent);
    color: #fff;
    font-size: var(--sw-font-size-small);
  }

  .empty {
    grid-column: 1 / -1;
    color: var(--sw-text-faint);
  }

  .spacer {
    flex: 1;
  }
</style>
