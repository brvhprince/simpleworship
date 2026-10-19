<!--
  @component
  The CCLI song usage report: every song shown live in a period, and on how
  many days. Export it as CSV for CCLI's reporting site or a spreadsheet.
  A song counts once per day, however often it is shown.
-->
<script lang="ts">
  import type { SongUsage } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { save } from "@tauri-apps/plugin-dialog";
  import { exportUsageCsv, usageReport } from "../lib/api.ts";
  import { dialogs } from "../lib/dialogs.svelte.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";

  /** A date as `YYYY-MM-DD`, in local time. */
  function isoDate(date: Date): string {
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    return `${date.getFullYear()}-${month}-${day}`;
  }

  // CCLI reports usually cover six months; start with the last one.
  const today = new Date();
  let from = $state(isoDate(new Date(today.getFullYear(), today.getMonth() - 1, today.getDate())));
  let to = $state(isoDate(today));
  let rows: SongUsage[] = $state.raw([]);

  $effect(() => {
    usageReport(from, to)
      .then((report) => (rows = report))
      .catch((error) => engine.showError(error));
  });

  async function exportCsv() {
    const path = await save({
      defaultPath: t("usage.fileName", { from, to }),
      filters: [{ name: "CSV", extensions: ["csv"] }],
    });
    if (!path) return;
    try {
      await exportUsageCsv(from, to, path);
    } catch (error) {
      engine.showError(error);
    }
  }
</script>

<Dialog title={t("usage.title")} onclose={() => dialogs.close()}>
  <div class="range">
    <label>{t("usage.from")} <input type="date" bind:value={from}></label>
    <label>{t("usage.to")} <input type="date" bind:value={to}></label>
    <span class="hint">{t("usage.count", { count: rows.length })}</span>
  </div>

  <div class="table">
    <table>
      <thead>
        <tr>
          <th>{t("usage.songTitle")}</th>
          <th>{t("usage.author")}</th>
          <th>{t("usage.ccli")}</th>
          <th class="number">{t("usage.times")}</th>
          <th>{t("usage.lastUsed")}</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row (row.title + row.lastUsed)}
          <tr>
            <td>{row.title}</td>
            <td>{row.author}</td>
            <td class:missing={!row.ccliNumber}>{row.ccliNumber || "—"}</td>
            <td class="number">{row.times}</td>
            <td>{row.lastUsed}</td>
          </tr>
        {:else}
          <tr>
            <td colspan="5" class="empty">{t("usage.empty")}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  {#snippet actions()}
    <Button onclick={() => dialogs.close()}>{t("common.close")}</Button>
    <Button variant="primary" disabled={rows.length === 0} onclick={exportCsv}
      >{t("usage.export")}</Button
    >
  {/snippet}
</Dialog>

<style>
  .range {
    display: flex;
    align-items: center;
    gap: var(--sw-space-3);
    margin-bottom: var(--sw-space-3);
  }

  .hint {
    color: var(--sw-text-faint);
  }

  .table {
    overflow: auto;
    max-height: 60vh;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: var(--sw-space-1) var(--sw-space-2);
    text-align: left;
    border-bottom: 1px solid var(--sw-border);
  }

  th {
    position: sticky;
    top: 0;
    background: var(--sw-surface);
    color: var(--sw-text-muted);
    font-weight: 600;
  }

  .number {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .missing {
    color: var(--sw-text-faint);
  }

  .empty {
    padding: var(--sw-space-4);
    color: var(--sw-text-faint);
    text-align: center;
  }
</style>
