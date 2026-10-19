<!--
  @component
  How the screen changes for each kind of content, like EasyWorship: cut
  straight to scripture during the sermon, fade between song slides, or
  whatever suits the service. Saved, so it is the same next time.
-->
<script lang="ts">
  import type { Transition, TransitionSettings } from "@simpleworship/protocol";
  import { saveTransitions } from "../lib/api.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { type MessageKey, t } from "../lib/i18n/i18n.svelte.ts";

  const KINDS: [keyof TransitionSettings, MessageKey][] = [
    ["songs", "library.songs"],
    ["scripture", "library.scripture"],
    ["presentations", "library.presentations"],
    ["other", "transitions.other"],
  ];

  const settings = $derived(engine.transitions);

  function set(kind: keyof TransitionSettings, change: Partial<Transition>) {
    const next = { ...settings, [kind]: { ...settings[kind], ...change } };
    saveTransitions(next).catch((error) => engine.showError(error));
  }

  /** Cut for all four at once: the quickest setting for a live service. */
  function cutEverything() {
    const cut: Transition = { kind: "cut", durationMs: 0 };
    saveTransitions({ songs: cut, scripture: cut, presentations: cut, other: cut }).catch((error) =>
      engine.showError(error),
    );
  }
</script>

<div class="rows">
  {#each KINDS as [kind, label] (kind)}
    {@const transition = settings[kind]}
    <span class="kind">{t(label)}</span>
    <select
      value={transition.kind}
      aria-label={t("transitions.forKind", { kind: t(label) })}
      onchange={(e) => {
        const chosen = e.currentTarget.value as Transition["kind"];
        set(kind, {
          kind: chosen,
          durationMs: chosen === "fade" ? transition.durationMs || 400 : 0,
        });
      }}
    >
      <option value="cut">{t("toolbar.cut")}</option>
      <option value="fade">{t("toolbar.fade")}</option>
    </select>
    {#if transition.kind === "fade"}
      <span class="duration">
        <input
          type="number"
          min="0.1"
          max="10"
          step="0.1"
          value={transition.durationMs / 1000}
          aria-label={t("toolbar.fadeSeconds")}
          onchange={(e) => {
            const seconds = e.currentTarget.valueAsNumber;
            if (Number.isFinite(seconds)) {
              set(kind, { durationMs: Math.round(Math.min(Math.max(seconds, 0.1), 10) * 1000) });
            }
          }}
        >
        {t("toolbar.secondsUnit")}
      </span>
    {:else}
      <span></span>
    {/if}
  {/each}
</div>
<button type="button" class="all-cuts" onclick={cutEverything}>{t("transitions.allCuts")}</button>

<style>
  .rows {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .kind {
    color: var(--sw-text-muted);
  }

  .duration {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .duration input {
    width: 56px;
  }

  .all-cuts {
    margin-top: var(--sw-space-3);
    padding: 0;
    border: none;
    background: none;
    color: var(--sw-accent);
    cursor: pointer;
  }
</style>
