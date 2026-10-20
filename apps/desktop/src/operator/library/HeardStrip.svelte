<!--
  @component
  The references Listen heard, newest first, across the top of the
  Scripture tab: "John 3:16 · 5 s ago". Clicking one (or pressing H for
  the newest) shows it in the preview, in the tab's translations;
  double-clicking sends it live. It never goes live by itself.

  Hovering shows the words it was heard in, to tell a reading from a
  passing mention.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { type Heard, listen } from "../../lib/listen.svelte.ts";

  interface Props {
    /** Shows a heard reference in the preview. */
    onchoose: (heard: Heard) => void;
    /** Shows a heard reference live (a double-click). */
    ongolive: (heard: Heard) => void;
  }

  let { onchoose, ongolive }: Props = $props();

  // "5 s ago" keeps counting.
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  function ago(at: number): string {
    const seconds = Math.max(0, Math.round((now - at) / 1000));
    if (seconds < 60) return t("listen.secondsAgo", { count: seconds });
    return t("listen.minutesAgo", { count: Math.round(seconds / 60) });
  }
</script>

{#if listen.listening || listen.heard.length > 0}
  <section class="heard" aria-label={t("listen.heard")}>
    <span class="label" class:on={listen.listening}>
      {listen.listening ? t("listen.listening") : t("listen.heard")}
    </span>
    {#each listen.heard as heard, index (heard.reference)}
      <button
        type="button"
        class="chip"
        class:newest={index === 0}
        title={t("listen.saidHint", { said: heard.said })}
        onclick={() => onchoose(heard)}
        ondblclick={() => ongolive(heard)}
      >
        <strong>{heard.reference}</strong>
        <span class="ago">{ago(heard.at)}</span>
      </button>
    {:else}
      <span class="empty">{t("listen.nothingYet")}</span>
    {/each}
    {#if listen.heard.length > 0}
      <span class="spacer"></span>
      <Button onclick={() => listen.clear()}>{t("common.clear")}</Button>
    {/if}
  </section>
{/if}

<style>
  .heard {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-1);
    padding: var(--sw-space-1) var(--sw-space-2);
    border-bottom: 1px solid var(--sw-border);
    background: var(--sw-surface-raised);
  }

  .label {
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  /* A red dot while listening, as a recording light. */
  .label.on::before {
    content: "";
    display: inline-block;
    width: 0.5em;
    height: 0.5em;
    margin-right: 0.4em;
    border-radius: 50%;
    background: var(--sw-live);
    vertical-align: 0.1em;
  }

  .chip {
    display: inline-flex;
    align-items: baseline;
    gap: var(--sw-space-1);
    padding: 2px var(--sw-space-2);
    border: 1px solid var(--sw-border);
    border-radius: 999px;
    background: var(--sw-surface);
    color: var(--sw-text);
    font: inherit;
    cursor: pointer;
  }

  .chip:hover {
    background: var(--sw-surface-hover);
  }

  .chip.newest {
    border-color: var(--sw-preview);
  }

  .ago,
  .empty {
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }

  .spacer {
    flex: 1;
  }
</style>
