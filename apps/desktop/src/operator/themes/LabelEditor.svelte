<!--
  @component
  Settings for a line of small print: the scripture reference or the
  footer. Size, colour, font and alignment, and where it goes: a quick
  position (a corner or edge, or straight after the text), or drag its box
  on the preview.
-->
<script lang="ts">
  import type { FooterStyle, HorizontalAlign } from "@simpleworship/protocol";
  import { firstFamily, fontFamily } from "../../lib/fonts.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import {
    applyPreset,
    currentPreset,
    type EdgeSpacing,
    LABEL_PRESETS,
  } from "../../lib/label-placement.ts";
  import ColorField from "./ColorField.svelte";

  interface Props {
    /** What it is, for labels: "Reference" or "Footer". */
    name: string;
    value: FooterStyle;
    /** Where its quick positions put it. */
    spacing: EdgeSpacing;
    fonts: string[];
    onchange: (value: FooterStyle) => void;
  }

  let { name, value, spacing, fonts, onchange }: Props = $props();

  const preset = $derived(currentPreset(value, spacing));
  const font = $derived(firstFamily(value.fontFamily));

  function set<K extends keyof FooterStyle>(key: K, next: FooterStyle[K]) {
    onchange({ ...value, [key]: next });
  }
</script>

<div class="row">
  <label class="inline">
    <input
      type="checkbox"
      checked={value.visible}
      onchange={(e) => set("visible", e.currentTarget.checked)}
    >
    {t("label.show")}
  </label>
  <label class="inline">
    {t("label.size")}
    <input
      type="number"
      min="10"
      max="120"
      value={value.fontSize}
      onchange={(e) => set("fontSize", e.currentTarget.valueAsNumber || 28)}
    >
  </label>
  <label class="inline">
    <input
      type="checkbox"
      checked={value.bold}
      onchange={(e) => set("bold", e.currentTarget.checked)}
    >
    {t("label.bold")}
  </label>
</div>

<div class="row">
  <label class="inline">
    <input
      type="checkbox"
      checked={value.color === ""}
      onchange={(e) => set("color", e.currentTarget.checked ? "" : "#ffffff")}
    >
    {t("label.textColour")}
  </label>
  {#if value.color !== ""}
    <ColorField
      label={t("label.colour", { name })}
      value={value.color}
      onchange={(color) => set("color", color)}
    />
  {/if}
  <select
    value={font}
    aria-label={t("label.font", { name })}
    onchange={(e) =>
      set("fontFamily", e.currentTarget.value ? fontFamily(e.currentTarget.value) : "")}
  >
    <option value="">{t("label.textFont")}</option>
    {#if font && !fonts.includes(font)}
      <option value={font}>{font}</option>
    {/if}
    {#each fonts as family (family)}
      <option value={family}>{family}</option>
    {/each}
  </select>
  <select
    value={value.align ?? ""}
    aria-label={t("label.alignment", { name })}
    onchange={(e) => set("align", (e.currentTarget.value || null) as HorizontalAlign | null)}
  >
    <option value="">{t("label.likeText")}</option>
    <option value="left">{t("designer.left")}</option>
    <option value="center">{t("designer.centre")}</option>
    <option value="right">{t("designer.right")}</option>
  </select>
</div>

<fieldset class="presets" aria-label={t("label.position", { name })}>
  {#each LABEL_PRESETS as option (option.value)}
    <button
      type="button"
      class:active={preset === option.value}
      aria-pressed={preset === option.value}
      onclick={() => onchange(applyPreset(value, option.value, spacing))}
    >
      {t(option.label)}
    </button>
  {/each}
</fieldset>
{#if value.placement === "box"}
  <label class="inline hint">
    <input
      type="checkbox"
      checked={value.textBox.autoFit}
      onchange={(e) => set("textBox", { ...value.textBox, autoFit: e.currentTarget.checked })}
    >
    {t("label.shrink", { name })}
  </label>
{/if}

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  .inline {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  input[type="number"] {
    width: 70px;
  }

  .presets {
    margin: 0;
    padding: 0;
    border: none;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }

  .presets button {
    padding: 4px 6px;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    cursor: pointer;
    font-size: var(--sw-font-size-small);
  }

  /* "After the text" takes a row of its own. */
  .presets button:last-child {
    grid-column: 1 / -1;
  }

  .presets button.active {
    border-color: var(--sw-accent);
    background: var(--sw-accent-soft);
  }

  .hint {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
