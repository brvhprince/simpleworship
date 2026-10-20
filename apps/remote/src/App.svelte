<!--
  @component
  The phone remote. Shows pairing until this phone is allowed, then the
  screens its role may use, as tabs along the bottom:

  | Role           | Screens                                            |
  | -------------- | -------------------------------------------------- |
  | Full control   | Live, Schedule, Bible, Songs, Alerts, Stage        |
  | Presenter      | Live                                               |
  | Worship leader | Live, Songs (the songs in the schedule)            |
  | Alerts         | Alerts                                             |
  | Stage view     | the stage display, full screen                     |
  | Mirror         | the stage display (mirrors are meant for computers) |
-->
<script lang="ts">
  import { i18n, type MessageKey, t } from "@simpleworship/i18n";
  import type { Role } from "@simpleworship/protocol";
  import { setCameraUse, setMediaUrls } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { remote } from "./lib/remote.svelte.ts";
  import Alerts from "./screens/Alerts.svelte";
  import Live from "./screens/Live.svelte";
  import Pairing from "./screens/Pairing.svelte";
  import Schedule from "./screens/Schedule.svelte";
  import Scripture from "./screens/Scripture.svelte";
  import Songs from "./screens/Songs.svelte";
  import StageControls from "./screens/StageControls.svelte";
  import StageDisplay from "./screens/StageDisplay.svelte";

  type Screen = "live" | "schedule" | "scripture" | "songs" | "alerts" | "stage";

  const SCREENS: Record<Role, Screen[]> = {
    full: ["live", "schedule", "scripture", "songs", "alerts", "stage"],
    presenter: ["live"],
    worshipLeader: ["live", "songs"],
    alerts: ["alerts"],
    stage: [],
    mirror: [],
  };

  const LABELS: Record<Screen, MessageKey> = {
    live: "remote.tabLive",
    schedule: "remote.tabSchedule",
    scripture: "remote.tabBible",
    songs: "remote.tabSongs",
    alerts: "remote.tabAlerts",
    stage: "remote.tabStage",
  };

  let screen: Screen = $state("live");

  const screens = $derived(remote.role ? SCREENS[remote.role] : []);
  const shown = $derived(screens.includes(screen) ? screen : (screens[0] ?? "live"));

  setMediaUrls({
    file: (media) => remote.mediaUrl(media),
    thumbnail: (media) => remote.thumbnailUrl(media),
  });

  // A camera belongs to the computer: phones show its symbol and name.
  setCameraUse({ live: false, placeholder: true, explain: false });

  onMount(() => {
    const stop = i18n.listen();
    if (remote.token) remote.connect();
    return stop;
  });
</script>

<main class="remote">
  {#if remote.status === "pairing" || remote.status === "waiting"}
    <Pairing />
  {:else if remote.status === "removed"}
    <section class="message">
      <h1>{t("remote.removedTitle")}</h1>
      <p>{t("remote.removedText")}</p>
      <button type="button" class="primary" onclick={() => remote.forget()}>
        {t("remote.pairAgain")}
      </button>
    </section>
  {:else if !remote.role}
    <section class="message"><p>{t("remote.connecting")}</p></section>
  {:else}
    {#if remote.status === "offline"}
      <div class="offline" role="status">{t("remote.reconnecting")}</div>
    {/if}

    {#if remote.role === "stage" || remote.role === "mirror"}
      <StageDisplay />
    {:else}
      <div class="screen">
        {#if shown === "live"}
          <Live />
        {:else if shown === "schedule"}
          <Schedule />
        {:else if shown === "scripture"}
          <Scripture />
        {:else if shown === "songs"}
          {#if remote.role === "worshipLeader"}
            <Schedule songsOnly />
          {:else}
            <Songs />
          {/if}
        {:else if shown === "alerts"}
          <Alerts />
        {:else if shown === "stage"}
          <StageControls />
        {/if}
      </div>

      {#if screens.length > 1}
        <nav class="tabs" aria-label={t("remote.screens")}>
          {#each screens as item (item)}
            <button
              type="button"
              class:active={item === shown}
              aria-current={item === shown ? "page" : undefined}
              onclick={() => (screen = item)}
            >
              {t(LABELS[item])}
            </button>
          {/each}
        </nav>
      {/if}
    {/if}
  {/if}

  {#if remote.error}
    <div class="error" role="alert">{remote.error}</div>
  {/if}
</main>

<style>
  .remote {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding-top: env(safe-area-inset-top);
  }

  .screen {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .tabs {
    display: flex;
    border-top: 1px solid var(--sw-border);
    background: var(--sw-surface);
    padding-bottom: env(safe-area-inset-bottom);
  }

  .tabs button {
    flex: 1;
    min-height: 54px;
    border: none;
    background: none;
    color: var(--sw-text-muted);
    font-size: 13px;
  }

  .tabs button.active {
    color: var(--sw-accent);
    font-weight: 600;
  }

  .message {
    margin: auto;
    padding: 24px;
    max-width: 420px;
    text-align: center;
    line-height: 1.5;
  }

  .primary {
    min-height: 48px;
    padding: 0 24px;
    border: none;
    border-radius: 10px;
    background: var(--sw-accent);
    color: #fff;
  }

  .offline {
    padding: 8px 16px;
    background: rgba(245, 159, 0, 0.2);
    color: #f59f00;
    text-align: center;
    font-size: 14px;
  }

  .error {
    position: fixed;
    left: 12px;
    right: 12px;
    bottom: calc(70px + env(safe-area-inset-bottom));
    padding: 12px 16px;
    border-radius: 10px;
    background: var(--sw-live);
    color: #fff;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }
</style>
