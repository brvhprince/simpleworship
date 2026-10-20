<!--
  @component
  Settings › Stream: how the stream output (the browser source, NDI and the
  stream window) shows each kind of content, and what is behind the words.
  Changes apply and are saved at once, like the transitions.
-->
<script lang="ts">
  import type { StreamBackdrop, StreamLook, StreamSettings } from "@simpleworship/protocol";
  import { saveStream } from "../lib/api.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { type MessageKey, t } from "../lib/i18n/i18n.svelte.ts";
  import NdiSettings from "./NdiSettings.svelte";
  import StreamLinks from "./StreamLinks.svelte";

  type Kind = "songs" | "scripture" | "presentations" | "other";

  const KINDS: { kind: Kind; label: MessageKey }[] = [
    { kind: "songs", label: "library.songs" },
    { kind: "scripture", label: "library.scripture" },
    { kind: "presentations", label: "library.presentations" },
    { kind: "other", label: "transitions.other" },
  ];

  const BACKDROPS: { backdrop: StreamBackdrop; label: MessageKey }[] = [
    { backdrop: "transparent", label: "stream.transparent" },
    { backdrop: "green", label: "stream.green" },
    { backdrop: "black", label: "stream.black" },
    { backdrop: "main", label: "stream.mainBackground" },
  ];

  const settings = $derived(engine.stream);
  const themes = $derived(engine.themes.themes);

  /** A look as a `<select>` value: a theme id, "main" or "hidden". */
  function selectValue(look: StreamLook): string {
    if (look.type === "theme") return look.themeId;
    return look.type === "hidden" ? "hidden" : "main";
  }

  function lookFor(value: string): StreamLook {
    if (value === "main") return { type: "sameAsMain" };
    if (value === "hidden") return { type: "hidden" };
    return { type: "theme", themeId: value };
  }

  function change(update: Partial<StreamSettings>) {
    if (!settings) return;
    saveStream({ ...settings, ...update }).catch((error) => engine.showError(error));
  }
</script>

{#if settings}
  <p class="hint">{t("stream.intro")}</p>
  <div class="looks">
    {#each KINDS as { kind, label } (kind)}
      <label for="stream-{kind}">{t(label)}</label>
      <select
        id="stream-{kind}"
        value={selectValue(settings[kind])}
        onchange={(e) => change({ [kind]: lookFor(e.currentTarget.value) })}
      >
        {#each themes as record (record.id)}
          <option value={record.id}>{t("stream.withTheme", { name: record.theme.name })}</option>
        {/each}
        <option value="main">{t("stream.sameAsMain")}</option>
        <option value="hidden">{t("stream.hidden")}</option>
      </select>
    {/each}

    <label for="stream-backdrop">{t("stream.backdrop")}</label>
    <select
      id="stream-backdrop"
      value={settings.backdrop}
      onchange={(e) => change({ backdrop: e.currentTarget.value as StreamBackdrop })}
    >
      {#each BACKDROPS as { backdrop, label } (backdrop)}
        <option value={backdrop}>{t(label)}</option>
      {/each}
    </select>
  </div>
  <label class="option">
    <input
      type="checkbox"
      checked={settings.showAlerts}
      onchange={(e) => change({ showAlerts: e.currentTarget.checked })}
    >
    {t("stream.showAlerts")}
  </label>
  <p class="hint">{t("stream.panicHint")}</p>
  <StreamLinks />
  <NdiSettings />
{/if}

<style>
  .looks {
    display: grid;
    grid-template-columns: max-content minmax(0, 320px);
    align-items: center;
    gap: var(--sw-space-2) var(--sw-space-3);
    margin-bottom: var(--sw-space-3);
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    margin-bottom: var(--sw-space-2);
  }

  .hint {
    margin: 0 0 var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
