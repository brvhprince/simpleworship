<!--
  @component
  A colour: a swatch that opens the system colour picker, a text box
  for any CSS colour (so transparency such as `rgba(0, 0, 0, 0.6)` works),
  and a presets palette with popular worship colors and custom saved colors.
-->
<script lang="ts">
  import { t } from "../../lib/i18n/i18n.svelte.ts";

  interface Props {
    value: string;
    label: string;
    onchange: (value: string) => void;
  }

  let { value, label, onchange }: Props = $props();

  const DEFAULT_PRESETS = [
    "#ffffff",
    "#f8fafc",
    "#e2e8f0",
    "#94a3b8",
    "#334155",
    "#000000",
    "#fbbf24",
    "#f59e0b",
    "#ef4444",
    "#ec4899",
    "#8b5cf6",
    "#3b82f6",
    "#06b6d4",
    "#10b981",
    "#84cc16",
    "#fdba74",
  ];

  let open = $state(false);
  let root: HTMLElement;
  let customPresets = $state<string[]>(loadCustomPresets());

  function loadCustomPresets(): string[] {
    try {
      if (typeof localStorage === "undefined") return [];
      const raw = localStorage.getItem("simpleworship.color_presets");
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  }

  function saveCurrentPreset() {
    const trimmed = value.trim();
    if (!trimmed || DEFAULT_PRESETS.includes(trimmed) || customPresets.includes(trimmed)) return;
    customPresets = [...customPresets, trimmed];
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("simpleworship.color_presets", JSON.stringify(customPresets));
      }
    } catch {}
  }

  function removeCustomPreset(preset: string) {
    customPresets = customPresets.filter((p) => p !== preset);
    try {
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("simpleworship.color_presets", JSON.stringify(customPresets));
      }
    } catch {}
  }

  function handleWindowPointer(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) {
      open = false;
    }
  }

  $effect(() => {
    if (open) {
      window.addEventListener("pointerdown", handleWindowPointer);
      return () => window.removeEventListener("pointerdown", handleWindowPointer);
    }
  });

  /** The picker only understands `#rrggbb`; show black for anything else. */
  const swatch = $derived(/^#[0-9a-f]{6}$/i.test(value) ? value : "#000000");
</script>

<span class="color-field" bind:this={root}>
  <input
    type="color"
    value={swatch}
    aria-label={t("colour.picker", { label })}
    oninput={(e) => onchange(e.currentTarget.value)}
  >
  <input
    class="text"
    {value}
    aria-label={label}
    spellcheck="false"
    onchange={(e) => onchange(e.currentTarget.value.trim())}
  >
  <button
    type="button"
    class="palette-btn"
    class:active={open}
    title={t("colour.palette")}
    aria-label={t("colour.palette")}
    onclick={() => (open = !open)}
  >
    <svg
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <title>{t("colour.palette")}</title>
      <circle cx="13.5" cy="6.5" r=".5" fill="currentColor" />
      <circle cx="17.5" cy="10.5" r=".5" fill="currentColor" />
      <circle cx="8.5" cy="7.5" r=".5" fill="currentColor" />
      <circle cx="6.5" cy="12.5" r=".5" fill="currentColor" />
      <path
        d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.563-2.512 5.563-5.563C22 6.5 17.5 2 12 2z"
      />
    </svg>
  </button>

  {#if open}
    <div class="palette-popover" role="dialog" aria-label={t("colour.palette")}>
      <span class="palette-title">{t("colour.defaultPresets")}</span>
      <div class="preset-grid">
        {#each DEFAULT_PRESETS as preset (preset)}
          <button
            type="button"
            class="swatch-btn"
            style:background={preset}
            title={preset}
            onclick={() => {
              onchange(preset);
              open = false;
            }}
          ></button>
        {/each}
      </div>

      {#if customPresets.length > 0}
        <span class="palette-title">{t("colour.customPresets")}</span>
        <div class="preset-grid">
          {#each customPresets as preset (preset)}
            <div class="custom-swatch-wrapper">
              <button
                type="button"
                class="swatch-btn"
                style:background={preset}
                title={preset}
                onclick={() => {
                  onchange(preset);
                  open = false;
                }}
              ></button>
              <button
                type="button"
                class="remove-custom-btn"
                title="Remove"
                onclick={(e) => {
                  e.stopPropagation();
                  removeCustomPreset(preset);
                }}
              >
                ×
              </button>
            </div>
          {/each}
        </div>
      {/if}

      <button
        type="button"
        class="save-preset-btn"
        onclick={saveCurrentPreset}
        title={t("colour.savePreset")}
      >
        + {t("colour.savePreset")}
      </button>
    </div>
  {/if}
</span>

<style>
  .color-field {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
    position: relative;
  }

  input[type="color"] {
    width: 28px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--sw-border);
    background: none;
    border-radius: var(--sw-radius);
    cursor: pointer;
  }

  .text {
    width: 130px;
    font-family: var(--sw-font-mono);
  }

  .palette-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: transparent;
    color: var(--sw-text-muted);
    cursor: pointer;
    transition:
      background 0.15s,
      color 0.15s;
  }

  .palette-btn:hover,
  .palette-btn.active {
    color: var(--sw-text);
    background: var(--sw-surface-sunken, rgba(255, 255, 255, 0.08));
    border-color: var(--sw-accent);
  }

  .palette-popover {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 200;
    width: 196px;
    padding: var(--sw-space-2);
    background: var(--sw-surface, #1e1e24);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
  }

  .palette-title {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--sw-text-muted);
  }

  .preset-grid {
    display: grid;
    grid-template-columns: repeat(8, 18px);
    gap: 4px;
  }

  .swatch-btn {
    width: 18px;
    height: 18px;
    border-radius: 3px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    cursor: pointer;
    padding: 0;
    transition:
      transform 0.1s,
      border-color 0.1s;
  }

  .swatch-btn:hover {
    transform: scale(1.15);
    border-color: #ffffff;
    z-index: 1;
  }

  .custom-swatch-wrapper {
    position: relative;
    display: inline-block;
  }

  .remove-custom-btn {
    position: absolute;
    top: -5px;
    right: -5px;
    width: 12px;
    height: 12px;
    line-height: 10px;
    font-size: 10px;
    background: #000;
    color: #fff;
    border: 1px solid var(--sw-border);
    border-radius: 50%;
    cursor: pointer;
    padding: 0;
    display: none;
  }

  .custom-swatch-wrapper:hover .remove-custom-btn {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .save-preset-btn {
    font-size: 11px;
    padding: 3px 6px;
    background: transparent;
    border: 1px dashed var(--sw-border);
    border-radius: 3px;
    color: var(--sw-text-muted);
    cursor: pointer;
    text-align: center;
    margin-top: 2px;
  }

  .save-preset-btn:hover {
    color: var(--sw-text);
    border-color: var(--sw-accent);
  }
</style>
