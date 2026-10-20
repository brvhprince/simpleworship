<!--
  @component
  Listen, in the toolbar: turn it on or off, choose the microphone and how
  accurate (and big) the speech model is, and see that it is hearing (the
  level and the last sentence). The first time, the model is downloaded.

  What it hears is written down on this computer and never sent anywhere;
  references it hears are offered in the Scripture tab.
-->
<script lang="ts">
  import type { SpeechModel } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { formatBytes } from "../lib/format.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { listen } from "../lib/listen.svelte.ts";

  const status = $derived(listen.status);
  const settings = $derived(status?.settings ?? { device: null, model: "standard" as const });
  const chosen = $derived(status?.models.find((info) => info.model === settings.model));
  const download = $derived(status?.download ?? null);

  const MODEL_NAMES: Record<SpeechModel, string> = $derived({
    fast: t("listen.modelFast"),
    standard: t("listen.modelStandard"),
    accurate: t("listen.modelAccurate"),
  });

  onMount(() => {
    void listen.findDevices();
  });

  function chooseDevice(device: string) {
    void listen.configure({ ...settings, device: device || null });
  }

  function chooseModel(model: SpeechModel) {
    void listen.configure({ ...settings, model });
  }
</script>

<div class="listen">
  <p class="hint">{t("listen.hint")}</p>

  <label class="field">
    {t("listen.microphone")}
    <select value={settings.device ?? ""} onchange={(e) => chooseDevice(e.currentTarget.value)}>
      <option value="">{t("listen.defaultMicrophone")}</option>
      {#each listen.devices as device (device.id)}
        <option value={device.id}>{device.name}</option>
      {/each}
    </select>
  </label>

  <label class="field">
    {t("listen.accuracy")}
    <select
      value={settings.model}
      disabled={download !== null}
      onchange={(e) => chooseModel(e.currentTarget.value as SpeechModel)}
    >
      {#each status?.models ?? [] as info (info.model)}
        <option value={info.model}>
          {MODEL_NAMES[info.model]}
          {info.ready ? "" : ` · ${formatBytes(info.sizeBytes)}`}
        </option>
      {/each}
    </select>
  </label>

  {#if download}
    <div class="download">
      <progress max="1" value={download.progress}></progress>
      <span>{t("listen.downloading", { percent: Math.round(download.progress * 100) })}</span>
    </div>
  {:else if chosen && !chosen.ready}
    <p class="hint">{t("listen.needsModel", { size: formatBytes(chosen.sizeBytes) })}</p>
    <Button variant="primary" onclick={() => listen.download(settings.model)}>
      {t("listen.download")}
    </Button>
  {:else if status}
    <div class="row">
      {#if status.listening}
        <Button variant="live" active onclick={() => listen.stop()}>{t("listen.stop")}</Button>
      {:else}
        <Button variant="primary" disabled={status.starting} onclick={() => listen.start()}>
          {status.starting ? t("listen.starting") : t("listen.start")}
        </Button>
      {/if}
      <meter
        class="level"
        min="0"
        max="1"
        low="0.15"
        value={listen.level}
        aria-label={t("listen.level")}
      ></meter>
    </div>
    {#if status.listening}
      <p class="words" aria-live="polite">{listen.words || t("listen.waiting")}</p>
    {/if}
  {/if}

  {#if status?.problem}
    <p class="problem">{status.problem}</p>
  {/if}

  <p class="hint small">{t("listen.privacy")}</p>
</div>

<style>
  .listen {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    width: 22rem;
    max-width: 100%;
  }

  p {
    margin: 0;
    line-height: 1.45;
  }

  .hint {
    color: var(--sw-text-muted);
  }

  .small {
    font-size: var(--sw-font-size-small);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
  }

  .row,
  .download {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .level,
  progress {
    flex: 1;
    min-width: 0;
  }

  .words {
    min-height: 2.9em;
    padding: var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: var(--sw-bg);
    font-style: italic;
  }

  .problem {
    color: var(--sw-live);
  }
</style>
