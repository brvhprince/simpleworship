<!--
  @component
  Chooses the themes and lower thirds to share, then where to save the
  `.swpack` file (see `lib/packs.svelte.ts`). The pictures and videos
  behind the chosen themes go in the file too; fonts do not.
-->
<script lang="ts">
  import { Button, Dialog } from "@simpleworship/ui";
  import { save } from "@tauri-apps/plugin-dialog";
  import { onMount, untrack } from "svelte";
  import { exportPack } from "../lib/api.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { lowerThirds } from "../lib/lower-thirds.svelte.ts";
  import { PACK_FILTERS, packs } from "../lib/packs.svelte.ts";
  import { prompts } from "../lib/prompts.svelte.ts";

  /** What is ticked, starting with what the dialog was opened with. */
  const start = untrack(() => packs.exporting);
  let themeIds = $state(new Set(start?.themeIds ?? []));
  let lowerThirdIds = $state(new Set(start?.lowerThirdIds ?? []));
  let saving = $state(false);

  const themes = $derived(engine.themes.themes);

  // Opened from the Themes tab, the lower thirds may not be loaded yet.
  onMount(() => {
    if (!lowerThirds.ready) void lowerThirds.load();
  });
  const count = $derived(themeIds.size + lowerThirdIds.size);

  /** Ticks or unticks `id` in `set`, returning the new set. */
  function toggle(set: Set<string>, id: string): Set<string> {
    const next = new Set(set);
    if (!next.delete(id)) next.add(id);
    return next;
  }

  /** Closes the dialog, unless it is saving (it closes itself when done). */
  function close() {
    if (!saving) packs.exporting = null;
  }

  async function exportChosen() {
    const path = await save({
      filters: PACK_FILTERS,
      defaultPath: `${t("pack.defaultName")}.swpack`,
    });
    if (!path) return;
    saving = true;
    try {
      await exportPack(path, [...themeIds], [...lowerThirdIds]);
      packs.exporting = null;
      await prompts.ask({
        title: t("pack.exportedTitle"),
        message: t("pack.exportedMessage"),
        buttons: [{ id: "ok", label: t("common.ok"), variant: "primary" }],
        cancel: "ok",
      });
    } catch (error) {
      engine.showError(error);
    } finally {
      saving = false;
    }
  }
</script>

<Dialog title={t("pack.exportTitle")} onclose={close}>
  <p class="hint">{t("pack.exportHint")}</p>
  <div class="lists">
    <fieldset>
      <legend>{t("pack.themes")}</legend>
      <div class="quick">
        <Button onclick={() => (themeIds = new Set(themes.map((record) => record.id)))}
          >{t("pack.all")}</Button
        >
        <Button onclick={() => (themeIds = new Set())}>{t("pack.none")}</Button>
      </div>
      {#each themes as record (record.id)}
        <label>
          <input
            type="checkbox"
            checked={themeIds.has(record.id)}
            onchange={() => (themeIds = toggle(themeIds, record.id))}
          >
          {record.theme.name}
        </label>
      {/each}
    </fieldset>

    <fieldset>
      <legend>{t("pack.lowerThirds")}</legend>
      <div class="quick">
        <Button onclick={() => (lowerThirdIds = new Set(lowerThirds.list.map((item) => item.id)))}
          >{t("pack.all")}</Button
        >
        <Button onclick={() => (lowerThirdIds = new Set())}>{t("pack.none")}</Button>
      </div>
      {#each lowerThirds.list as item (item.id)}
        <label>
          <input
            type="checkbox"
            checked={lowerThirdIds.has(item.id)}
            onchange={() => (lowerThirdIds = toggle(lowerThirdIds, item.id))}
          >
          {item.title}
        </label>
      {:else}
        <p class="hint">{t("lowerThird.empty")}</p>
      {/each}
    </fieldset>
  </div>

  {#snippet actions()}
    <span class="spacer"></span>
    <Button disabled={saving} onclick={close}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={count === 0 || saving} onclick={exportChosen}>
      {t("pack.export", { count })}
    </Button>
  {/snippet}
</Dialog>

<style>
  .lists {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sw-space-4);
    width: min(720px, 80vw);
    max-height: 55vh;
  }

  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    min-width: 0;
    margin: 0;
    padding: var(--sw-space-2) var(--sw-space-3);
    overflow-y: auto;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
  }

  legend {
    padding: 0 var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .quick {
    display: flex;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-1);
  }

  label {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .hint {
    margin: 0 0 var(--sw-space-3);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .spacer {
    flex: 1;
  }
</style>
