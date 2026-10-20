<!--
  @component
  The assistant's part of Settings: turn it on, choose the AI service
  (Claude, ChatGPT or Gemini), give it the church's API key for that
  service (checked, then kept in the computer's keychain), and choose how
  capable a model it uses. Says plainly what is sent, and when.
-->
<script lang="ts">
  import type { AssistantModel, AssistantProvider } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { openProjectPage } from "../lib/api.ts";
  import { assistant } from "../lib/assistant.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";

  /** Each service: its name, where its keys are made, and what one looks like. */
  const SERVICES: Record<
    AssistantProvider,
    {
      name: string;
      page: "anthropic-keys" | "openai-keys" | "gemini-keys";
      site: string;
      looks: string;
    }
  > = {
    anthropic: {
      name: "Claude (Anthropic)",
      page: "anthropic-keys",
      site: "console.anthropic.com",
      looks: "sk-ant-…",
    },
    openai: {
      name: "ChatGPT (OpenAI)",
      page: "openai-keys",
      site: "platform.openai.com",
      looks: "sk-…",
    },
    gemini: {
      name: "Gemini (Google)",
      page: "gemini-keys",
      site: "aistudio.google.com",
      looks: "AIza…",
    },
  };

  let key = $state("");
  let checking = $state(false);
  let problem: string | null = $state(null);

  const settings = $derived(
    assistant.status?.settings ?? {
      enabled: false,
      provider: "anthropic" as const,
      model: "fast" as const,
    },
  );
  const service = $derived(SERVICES[settings.provider]);
  const models = $derived(assistant.status?.models ?? null);

  onMount(() => {
    void assistant.load();
  });

  async function change(changes: {
    enabled?: boolean;
    provider?: AssistantProvider;
    model?: AssistantModel;
  }) {
    problem = null;
    key = "";
    try {
      await assistant.configure({ ...settings, ...changes });
    } catch (error) {
      problem = String(error);
    }
  }

  async function saveKey() {
    checking = true;
    problem = null;
    try {
      await assistant.setKey(key);
      key = "";
    } catch (error) {
      problem = String(error);
    } finally {
      checking = false;
    }
  }

  async function forget() {
    problem = null;
    try {
      await assistant.forgetKey();
    } catch (error) {
      problem = String(error);
    }
  }
</script>

<label class="option">
  <input
    type="checkbox"
    checked={settings.enabled}
    onchange={(e) => change({ enabled: e.currentTarget.checked })}
  >
  {t("assistant.enable")}
</label>
<p class="hint">{t("assistant.hint")}</p>

{#if settings.enabled}
  <label class="option">
    {t("assistant.service")}
    <select
      value={settings.provider}
      onchange={(e) => change({ provider: e.currentTarget.value as AssistantProvider })}
    >
      {#each Object.entries(SERVICES) as [id, info] (id)}
        <option value={id}>{info.name}</option>
      {/each}
    </select>
  </label>

  {#if assistant.status?.hasKey}
    <div class="option">
      <span>{t("assistant.keySaved", { service: service.name })}</span>
      <Button onclick={forget}>{t("assistant.forgetKey")}</Button>
    </div>
  {:else}
    <div class="option">
      <input
        class="key"
        type="password"
        autocomplete="off"
        spellcheck="false"
        placeholder={service.looks}
        aria-label={t("assistant.key", { service: service.name })}
        bind:value={key}
        onkeydown={(e) => e.key === "Enter" && saveKey()}
      >
      <Button variant="primary" disabled={checking || !key.trim()} onclick={saveKey}>
        {checking ? t("assistant.checking") : t("assistant.saveKey")}
      </Button>
    </div>
    <p class="hint">
      {t("assistant.keyHint")}
      <button type="button" class="link" onclick={() => openProjectPage(service.page)}>
        {service.site}
      </button>
    </p>
  {/if}

  <label class="option">
    {t("assistant.model")}
    <select
      value={settings.model}
      onchange={(e) => change({ model: e.currentTarget.value as AssistantModel })}
    >
      <option value="fast">{t("assistant.modelFast")}</option>
      <option value="better">{t("assistant.modelBetter")}</option>
    </select>
  </label>
  {#if models}
    <p class="hint">
      {t("assistant.uses", { model: settings.model === "fast" ? models.fast : models.better })}
    </p>
  {/if}
  <p class="hint">{t("assistant.privacy")}</p>
{/if}

{#if problem}
  <p class="problem">{problem}</p>
{/if}

<style>
  /* The same rows and notes as the rest of Settings. */
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

  .key {
    flex: 1;
    min-width: 0;
    font-family: var(--sw-font-mono);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--sw-accent-strong);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }

  .problem {
    color: var(--sw-live);
  }
</style>
