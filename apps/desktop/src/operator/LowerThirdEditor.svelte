<!--
  @component
  Edits one lower third (`lowerThirds.editing`): what it is for, its two
  lines, its style, side and colours, where it shows and how long it stays.
  The preview plays its entrance over a sample picture, again whenever the
  style or side changes, or on Play Again.
-->
<script lang="ts">
  import type {
    LowerThirdKind,
    LowerThirdShown,
    LowerThirdSide,
    LowerThirdStyle,
    LowerThirdTarget,
  } from "@simpleworship/protocol";
  import { LowerThirdLayer, now, ScaledCanvas } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { listFonts } from "../lib/api.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { lowerThirds } from "../lib/lower-thirds.svelte.ts";
  import { KIND_EXAMPLES, KIND_LABELS, STYLE_LABELS } from "./lower-third-labels.ts";
  import ColorField from "./themes/ColorField.svelte";

  const KINDS = Object.keys(KIND_LABELS) as LowerThirdKind[];
  const STYLES = Object.keys(STYLE_LABELS) as LowerThirdStyle[];
  const SIDES: [LowerThirdSide, Parameters<typeof t>[0]][] = [
    ["left", "lowerThird.side.left"],
    ["centre", "lowerThird.side.centre"],
    ["right", "lowerThird.side.right"],
  ];
  const TARGETS: [LowerThirdTarget, Parameters<typeof t>[0]][] = [
    ["both", "lowerThird.target.both"],
    ["screens", "lowerThird.target.screens"],
    ["stream", "lowerThird.target.stream"],
  ];
  /** How long it stays: until hidden, or some seconds. */
  const HOLDS = [0, 5, 8, 10, 15, 30];

  let fonts: string[] = $state([]);
  /** When the preview last started its entrance (the renderer's clock). */
  let playedAt = $state(now());

  const item = $derived(lowerThirds.editing);
  /** The lower third as the preview shows it, entering from `playedAt`. */
  const preview: LowerThirdShown | null = $derived(
    // A preview never reaches the engine: its showing is just when it last played.
    item && {
      showId: `preview-${playedAt}`,
      item: { ...item, hideAfterSeconds: null },
      startedAt: playedAt,
    },
  );

  onMount(() => {
    listFonts().then((list) => (fonts = list));
  });

  function replay() {
    playedAt = now();
  }

  // A new style or side shows how it moves in.
  $effect(() => {
    void item?.look.style;
    void item?.look.side;
    replay();
  });

  async function save(andShow: boolean) {
    const saved = await lowerThirds.saveEditing();
    if (saved && andShow) lowerThirds.show(saved);
  }
</script>

{#if item}
  <Dialog title={t("lowerThird.editTitle")} onclose={() => (lowerThirds.editing = null)}>
    <div class="editor">
      <div class="preview">
        <ScaledCanvas background="linear-gradient(135deg, #22314f 0%, #4a2140 55%, #12151d 100%)">
          {#if preview}
            {#key playedAt}
              <LowerThirdLayer shown={preview} />
            {/key}
          {/if}
        </ScaledCanvas>
        <Button onclick={replay}>{t("lowerThird.playAgain")}</Button>
        <p class="hint">{t("lowerThird.hint")}</p>
      </div>

      <!-- Locked while saving, so a change made meanwhile cannot be lost. -->
      <fieldset class="form" disabled={lowerThirds.busy}>
        <label class="field">
          {t("lowerThird.kind")}
          <select bind:value={item.kind}>
            {#each KINDS as kind (kind)}
              <option value={kind}>{t(KIND_LABELS[kind])}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          {t("lowerThird.title")}
          <input bind:value={item.title} placeholder={t(KIND_EXAMPLES[item.kind][0])}>
        </label>
        <label class="field">
          {t("lowerThird.subtitle")}
          <input bind:value={item.subtitle} placeholder={t(KIND_EXAMPLES[item.kind][1])}>
        </label>

        <div class="field">
          {t("lowerThird.style")}
          <div class="choices">
            {#each STYLES as style (style)}
              <Button active={item.look.style === style} onclick={() => (item.look.style = style)}>
                {t(STYLE_LABELS[style])}
              </Button>
            {/each}
          </div>
        </div>

        <div class="field">
          {t("lowerThird.side")}
          <div class="choices">
            {#each SIDES as [side, label] (side)}
              <Button active={item.look.side === side} onclick={() => (item.look.side = side)}>
                {t(label)}
              </Button>
            {/each}
          </div>
        </div>

        <div class="colours">
          <ColorField
            label={t("lowerThird.accent")}
            value={item.look.accent}
            onchange={(color) => (item.look.accent = color)}
          />
          <ColorField
            label={t("lowerThird.box")}
            value={item.look.background}
            onchange={(color) => (item.look.background = color)}
          />
          <ColorField
            label={t("lowerThird.text")}
            value={item.look.color}
            onchange={(color) => (item.look.color = color)}
          />
        </div>

        <label class="field">
          {t("lowerThird.font")}
          <select bind:value={item.look.fontFamily}>
            <option value="">{t("lowerThird.fontDefault")}</option>
            {#each fonts as font (font)}
              <option value={font}>{font}</option>
            {/each}
          </select>
        </label>

        <div class="row">
          <label class="field">
            {t("lowerThird.target")}
            <select bind:value={item.target}>
              {#each TARGETS as [target, label] (target)}
                <option value={target}>{t(label)}</option>
              {/each}
            </select>
          </label>
          <label class="field">
            {t("lowerThird.hideAfter")}
            <select
              value={item.hideAfterSeconds ?? 0}
              onchange={(event) => {
                const seconds = Number(event.currentTarget.value);
                item.hideAfterSeconds = seconds > 0 ? seconds : null;
              }}
            >
              {#each HOLDS as seconds (seconds)}
                <option value={seconds}>
                  {seconds === 0
                    ? t("lowerThird.hideManual")
                    : t("lowerThird.hideSeconds", { count: seconds })}
                </option>
              {/each}
            </select>
          </label>
        </div>
      </fieldset>
    </div>

    {#snippet actions()}
      {#if !lowerThirds.isNew}
        <Button disabled={lowerThirds.busy} onclick={() => lowerThirds.deleteEditing()}
          >{t("common.delete")}</Button
        >
      {/if}
      <span class="spacer"></span>
      <Button onclick={() => (lowerThirds.editing = null)}>{t("common.cancel")}</Button>
      <Button disabled={lowerThirds.busy || !item.title.trim()} onclick={() => save(false)}
        >{t("common.save")}</Button
      >
      <Button
        variant="primary"
        disabled={lowerThirds.busy || !item.title.trim()}
        onclick={() => save(true)}
      >
        {t("lowerThird.saveShow")}
      </Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .editor {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: var(--sw-space-4);
    width: min(1000px, 86vw);
  }

  .preview {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sw-space-2);
  }

  .preview :global(.canvas-frame),
  .preview > :first-child {
    width: 100%;
  }

  .form {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .field input,
  .field select {
    color: var(--sw-text);
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sw-space-1);
  }

  .colours,
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sw-space-3);
  }

  .row .field {
    flex: 1;
  }

  .hint {
    margin: 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .spacer {
    flex: 1;
  }
</style>
