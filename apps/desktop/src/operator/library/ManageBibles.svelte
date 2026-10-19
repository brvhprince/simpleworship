<!--
  @component
  Lists the installed Bibles, and lets the operator import, rename and
  delete them. Imports run in the background and take a few seconds.
-->
<script lang="ts">
  import type { BibleInfo } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { open } from "@tauri-apps/plugin-dialog";
  import { deleteBible, importBible, renameBible } from "../../lib/api.ts";
  import { bibles } from "../../lib/bibles.svelte.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();

  /** File types the importers understand (see `crates/sw-bible/src/import`). */
  const BIBLE_FILTERS = [
    {
      name: "Bible files",
      extensions: ["xml", "osis", "xmm", "zip", "usfm", "sfm", "usx"],
    },
  ];

  let importing: string | null = $state(null);

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
      title: "Delete Bible?",
      message: `Delete “${bible.name}” (${bible.abbreviation})? Schedules that use it keep their copy of the text.`,
      buttons: [
        { id: "cancel", label: "Cancel" },
        { id: "delete", label: "Delete", variant: "live" },
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

<Dialog title="Bibles" {onclose}>
  <table>
    <thead>
      <tr>
        <th>Short name</th>
        <th>Name</th>
        <th class="count">Verses</th>
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
              aria-label="Short name"
              onchange={(e) => rename(bible, bible.name, e.currentTarget.value)}
            >
          </td>
          <td>
            <input
              value={bible.name}
              aria-label="Name"
              onchange={(e) => rename(bible, e.currentTarget.value, bible.abbreviation)}
            >
            {#if bible.copyright}
              <div class="copyright" title={bible.copyright}>{bible.copyright}</div>
            {/if}
          </td>
          <td class="count">{bible.verseCount.toLocaleString()}</td>
          <td><Button onclick={() => remove(bible)}>Delete</Button></td>
        </tr>
      {:else}
        <tr>
          <td colspan="4" class="hint">No Bibles installed yet.</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <p class="licence">
    SimpleWorship includes the King James Version and the World English Bible, which are in the
    public domain. You can import OSIS, Zefania, OpenSong, USX and USFM files (or a zip of them).
    Most modern translations are copyrighted: only import Bibles you are licensed to show.
  </p>

  {#snippet actions()}
    {#if importing}
      <span class="importing">Importing {importing}…</span>
    {/if}
    <Button disabled={importing !== null} onclick={chooseAndImport}>Import Bible…</Button>
    <Button variant="primary" onclick={onclose}>Done</Button>
  {/snippet}
</Dialog>

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
