<!--
  @component
  What is in preview, with a big Go Live button: shown under the Schedule,
  Bible and Songs screens once something has been picked.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import { frameAt, ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { remote } from "../lib/remote.svelte.ts";

  const preview = $derived(remote.preview);
  const frame = $derived(preview ? frameAt(preview.presentation, preview.selected) : null);
</script>

{#if preview}
  <section class="preview">
    <header>
      <span class="badge">{t("pane.preview")}</span>
      <span class="title">{preview.presentation.title}</span>
    </header>
    {#if frame}
      <div class="slide">
        <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
      </div>
    {/if}
    <button type="button" class="go" onclick={() => void remote.dispatch({ type: "goLive" })}>
      {t("pane.goLive")}
    </button>
  </section>
{/if}

<style>
  .preview {
    position: sticky;
    bottom: 0;
    display: grid;
    grid-template-columns: 120px 1fr;
    grid-template-rows: auto auto;
    gap: 8px 12px;
    padding: 12px;
    border-top: 1px solid var(--sw-border);
    background: var(--sw-surface);
  }

  header {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .badge {
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--sw-preview);
    color: #fff;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .slide {
    border-radius: 6px;
    overflow: hidden;
  }

  .go {
    min-height: 56px;
    border: none;
    border-radius: 12px;
    background: var(--sw-live);
    color: #fff;
    font-size: 18px;
    font-weight: 700;
  }
</style>
