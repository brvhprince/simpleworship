<!--
  @component
  Lists the installed Bibles, and lets the operator import, rename and
  delete them, or download more (see `GetBibles`). Imports run in the
  background and take a few seconds.
-->
<script lang="ts">
  import type { BibleInfo } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { open } from "@tauri-apps/plugin-dialog";
  import { deleteBible, importBible, renameBible } from "../../lib/api.ts";
  import { bibles } from "../../lib/bibles.svelte.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";
  import GetBibles from "./GetBibles.svelte";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  /** File types the importers understand (see `crates/sw-bible/src/import`). */
  const BIBLE_FILTERS = [
    {
      name: t("bibles.files"),
      extensions: ["xml", "osis", "xmm", "zip", "usfm", "sfm", "usx"],
    },
  ];

  let importing: string | null = $state(null);
  /** Whether the Get More Bibles window is open. */
  let gettingMore = $state(false);

  async function chooseAndImport() {
    const path = await open({ filters: BIBLE_FILTERS, multiple: false });
    if (typeof path !== "string") return;

    importing = path.split(/[\\/]/).pop() ?? path;
    try {
      await importBible(path);
    } catch (error) {
      engine.showError(error);
    } finally {
      importing = null;
    }
  }

  async function rename(bible: BibleInfo, name: string, abbreviation: string) {
    if (!name.trim() || !abbreviation.trim()) return;
    try {
      await renameBible(bible.id, name, abbreviation);
    } catch (error) {
      engine.showError(error);
    }
  }

  async function remove(bible: BibleInfo) {
    const answer = await prompts.ask({
      title: t("bibles.deleteTitle"),
      message: t("bibles.deleteMessage", { name: bible.name, abbreviation: bible.abbreviation }),
      buttons: [
        { id: "cancel", label: t("common.cancel") },
        { id: "delete", label: t("common.delete"), variant: "live" },
      ],
      cancel: "cancel",
    });
    if (answer !== "delete") return;
    try {
      await deleteBible(bible.id);
    } catch (error) {
      engine.showError(error);
    }
  }
</script>

<Dialog title={t("bibles.title")} {onclose}>
  <table>
    <thead>
      <tr>
        <th>{t("bibles.shortName")}</th>
        <th>{t("common.name")}</th>
        <th class="count">{t("bibles.verses")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each bibles.list as bible (bible.id)}
        <tr>
          <td>
            <input
              class="abbreviation"
              value={bible.abbreviation}
              aria-label={t("bibles.shortName")}
              onchange={(e) => rename(bible, bible.name, e.currentTarget.value)}
            >
          </td>
          <td>
            <input
              value={bible.name}
              aria-label={t("common.name")}
              onchange={(e) => rename(bible, e.currentTarget.value, bible.abbreviation)}
            >
            {#if bible.copyright}
              <div class="copyright" title={bible.copyright}>{bible.copyright}</div>
            {/if}
          </td>
          <td class="count">{bible.verseCount.toLocaleString()}</td>
          <td><Button onclick={() => remove(bible)}>{t("common.delete")}</Button></td>
        </tr>
      {:else}
        <tr>
          <td colspan="4" class="hint">{t("bibles.none")}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <p class="licence">
    {t("bibles.licence")}
  </p>

  {#snippet actions()}
    {#if importing}
      <span class="importing">{t("bibles.importing", { file: importing })}</span>
    {/if}
    <Button onclick={() => (gettingMore = true)}>{t("bibles.getMore")}</Button>
    <Button disabled={importing !== null} onclick={chooseAndImport}>{t("bibles.import")}</Button>
    <Button variant="primary" onclick={onclose}>{t("common.done")}</Button>
  {/snippet}
</Dialog>

{#if gettingMore}
  <GetBibles onclose={() => (gettingMore = false)} />
{/if}

<style>
  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-weight: 600;
    color: var(--sw-text-muted);
    padding: var(--sw-space-1) var(--sw-space-2);
  }

  td {
    padding: var(--sw-space-1) var(--sw-space-2);
    vertical-align: top;
    border-top: 1px solid var(--sw-border);
  }

  td input {
    width: 100%;
  }

  .abbreviation {
    width: 90px;
  }

  .count {
    text-align: right;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .copyright {
    margin-top: 2px;
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 520px;
  }

  .hint {
    color: var(--sw-text-faint);
  }

  .licence {
    margin: var(--sw-space-4) 0 0;
    color: var(--sw-text-muted);
    line-height: 1.5;
  }

  .importing {
    margin-right: auto;
    color: var(--sw-text-muted);
  }
</style>
