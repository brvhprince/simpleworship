<!--
  @component
  Edits a background: a colour, a two-colour gradient, an image or a video
  from the media library.
-->
<script lang="ts">
  import type { Background, MediaFit, MediaItem } from "@simpleworship/protocol";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import ColorField from "./ColorField.svelte";
  import MediaPicker from "./MediaPicker.svelte";

  interface Props {
    value: Background;
    onchange: (background: Background) => void;
  }

  let { value, onchange }: Props = $props();

  /** "image"/"video" chosen in the select before a file is picked. */
  let pendingType: "image" | "video" | null = $state(null);
  const shownType = $derived(pendingType ?? value.type);

  // A different background from outside (another slide or theme) starts
  // with nothing pending.
  $effect.pre(() => {
    void value;
    pendingType = null;
  });

  /** Switching type keeps what can be kept (colours, the chosen media). */
  function setType(type: Background["type"]) {
    // Leaving image/video before a file was picked: nothing is pending now.
    pendingType = null;
    // Back to the type it already is: keep its colours as they were.
    if (type === value.type) return;
    switch (type) {
      case "color":
        onchange({ type, color: value.type === "linearGradient" ? value.from : "#000000" });
        break;
      case "linearGradient":
        onchange({
          type,
          from: value.type === "color" ? value.color : "#0b1a33",
          to: "#000000",
          angleDeg: 180,
        });
        break;
      case "image":
      case "video":
        // Nothing to show until media is picked; keep the old background.
        pendingType = type;
        break;
    }
  }

  function pick(media: MediaItem) {
    pendingType = null;
    const reference = { id: media.id, file: media.file };
    // Keep the fade when swapping one picture or video for another.
    const opacity = value.type === "image" || value.type === "video" ? value.opacity : 1;
    const fit: MediaFit = "cover";
    if (media.kind === "video") {
      onchange({ type: "video", media: reference, fit, loop: true, playAudio: false, opacity });
    } else {
      onchange({ type: "image", media: reference, fit, opacity });
    }
  }
</script>

<div class="background-editor">
  <select
    value={shownType}
    aria-label={t("background.type")}
    onchange={(e) => setType(e.currentTarget.value as Background["type"])}
  >
    <option value="color">{t("background.colour")}</option>
    <option value="linearGradient">{t("background.gradient")}</option>
    <option value="image">{t("background.image")}</option>
    <option value="video">{t("background.video")}</option>
  </select>

  {#if shownType === "color" && value.type === "color"}
    <ColorField
      label={t("background.colour")}
      value={value.color}
      onchange={(color) => onchange({ ...value, color })}
    />
  {:else if shownType === "linearGradient" && value.type === "linearGradient"}
    <ColorField
      label={t("background.from")}
      value={value.from}
      onchange={(from) => onchange({ ...value, from })}
    />
    <ColorField
      label={t("background.to")}
      value={value.to}
      onchange={(to) => onchange({ ...value, to })}
    />
    <label class="inline">
      {t("background.angle")}
      <input
        type="number"
        min="0"
        max="360"
        value={value.angleDeg}
        onchange={(e) => onchange({ ...value, angleDeg: e.currentTarget.valueAsNumber || 0 })}
      >°
    </label>
  {:else if shownType === "image" || shownType === "video"}
    <MediaPicker
      kind={shownType}
      value={value.type === "image" || value.type === "video" ? value.media : null}
      onchange={pick}
    />
    {#if value.type === "image" || value.type === "video"}
      <label class="inline">
        {t("background.fit")}
        <select
          value={value.fit}
          onchange={(e) => onchange({ ...value, fit: e.currentTarget.value as MediaFit })}
        >
          <option value="cover">{t("background.cover")}</option>
          <option value="contain">{t("background.contain")}</option>
          <option value="stretch">{t("background.stretch")}</option>
        </select>
      </label>
      <label class="inline" title={t("background.opacityHint")}>
        {t("background.opacity")}
        <input
          type="range"
          min="0.05"
          max="1"
          step="0.05"
          value={value.opacity}
          oninput={(e) => onchange({ ...value, opacity: e.currentTarget.valueAsNumber })}
        >
        <span class="value">{Math.round(value.opacity * 100)}%</span>
      </label>
    {/if}
    {#if value.type === "video"}
      <label class="inline">
        <input
          type="checkbox"
          checked={value.loop}
          onchange={(e) => onchange({ ...value, loop: e.currentTarget.checked })}
        >
        {t("common.loop")}
      </label>
      <label class="inline">
        <input
          type="checkbox"
          checked={value.playAudio}
          onchange={(e) => onchange({ ...value, playAudio: e.currentTarget.checked })}
        >
        {t("background.playSound")}
      </label>
    {/if}
  {/if}
</div>

<style>
  .background-editor {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .inline {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  input[type="number"] {
    width: 64px;
  }

  .value {
    min-width: 3em;
    font-variant-numeric: tabular-nums;
  }
</style>
