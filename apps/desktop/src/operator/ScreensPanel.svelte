<!--
  @component
  The toolbar's Screens panel: which screen each output goes on, and
  switching it on or off. Choosing another screen for an output that is on
  moves it there at once.
-->
<script lang="ts">
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { OUTPUTS, outputs, WINDOWED } from "../lib/outputs.svelte.ts";
</script>

<ul class="screens">
  {#each OUTPUTS as output (output.kind)}
    {@const on = outputs.isOpen(output.kind)}
    <li>
      <div class="name">
        <strong>{t(output.name)}</strong>
        <span class="hint">{t(output.hint)}</span>
      </div>
      <select
        value={outputs.screens[output.kind]}
        aria-label={t("screens.screenFor", { output: t(output.name) })}
        onfocus={() => outputs.refreshMonitors()}
        onchange={(e) => outputs.choose(output.kind, e.currentTarget.value)}
      >
        <option value={WINDOWED}>{t("toolbar.inWindow")}</option>
        {#each outputs.monitors as monitor (monitor.index)}
          <option value={String(monitor.index)}>
            {monitor.name}
            ({monitor.width}×{monitor.height}){monitor.isPrimary ? t("toolbar.mainScreen") : ""}
          </option>
        {/each}
      </select>
      <button
        type="button"
        class="switch"
        class:on
        role="switch"
        aria-checked={on}
        aria-label={t(output.name)}
        onclick={() => (on ? outputs.hide(output.kind) : outputs.show(output.kind))}
      >
        <span class="knob"></span>
      </button>
    </li>
  {/each}
</ul>

<style>
  .screens {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-3);
  }

  li {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-areas:
      "name switch"
      "select select";
    gap: var(--sw-space-1) var(--sw-space-2);
    align-items: center;
  }

  .name {
    grid-area: name;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .hint {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  select {
    grid-area: select;
    min-width: 0;
  }

  .switch {
    grid-area: switch;
    position: relative;
    width: 38px;
    height: 22px;
    padding: 0;
    border: 1px solid var(--sw-border);
    border-radius: 11px;
    background: var(--sw-surface-raised);
    cursor: pointer;
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--sw-text-muted);
    transition: transform 0.15s;
  }

  .switch.on {
    border-color: var(--sw-live);
    background: var(--sw-live);
  }

  .switch.on .knob {
    transform: translateX(16px);
    background: #fff;
  }
</style>
