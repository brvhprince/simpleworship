<!--
  @component
  Background music: the player bar in the operator window, and the hidden
  `<audio>` element that actually plays (on this computer, so it works even
  with no projector window open).

  The engine holds the playlist and the position (an anchor plus the clock,
  like video); this element follows it, fades out when asked, and tells the
  engine when a track ends.

  Only this one element plays the music, so once it is playing it keeps its
  own time: it moves only when the operator changes something (a seek, a
  new track). Correcting it against the clock would make it seek whenever
  it paused to load, which empties its buffer and makes it stutter.
-->
<script lang="ts">
  import { getMediaUrls, now as serviceNow } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { formatDuration } from "../lib/format.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";

  /** How far off the operator's own changes (a seek) may land, in seconds. */
  const OPERATOR_SEEK_S = 0.25;

  const urls = getMediaUrls();
  let element: HTMLAudioElement;
  /** The fade already reported as finished, so the pause is sent once. */
  let finishedFade: number | null = null;
  let now = $state(serviceNow());
  /** Where the element really is, in milliseconds, for the position display. */
  let playedMs: number | null = $state(null);
  /** The timeline last acted on, compared by value (see `follow`). */
  let handled = "";

  const audio = $derived(engine.audio);
  const track = $derived(audio?.playlist[audio.index] ?? null);
  const position = $derived.by(() => {
    if (!audio) return 0;
    if (playedMs !== null) return playedMs;
    const elapsed = audio.playing ? Math.max(0, now - audio.anchor) : 0;
    return audio.positionMs + elapsed;
  });

  onMount(() => {
    const timer = setInterval(() => {
      now = serviceNow();
      follow();
    }, 250);
    return () => clearInterval(timer);
  });

  // React straight away to play, pause, seek, track and volume changes.
  $effect(() => {
    void audio;
    follow();
  });

  /** Brings the element in line with the engine's state. */
  function follow() {
    // A joined computer shows the music's controls; the main computer
    // plays it (and moves on to the next track).
    if (link.joined) {
      element?.pause();
      playedMs = null;
      return;
    }
    if (!element || !audio || !track) {
      element?.pause();
      return;
    }
    const src = urls.file({ id: track.id, file: track.file });
    if (element.getAttribute("src") !== src) {
      element.src = src;
      handled = "";
    }

    // Move the playhead only when the operator changed the timeline. Every
    // engine event brings a fresh copy of the same state, hence by value.
    const timeline = `${audio.index}:${audio.positionMs}@${audio.anchor}:${audio.playing}`;
    if (timeline !== handled) {
      handled = timeline;
      const target =
        (audio.playing
          ? audio.positionMs + Math.max(0, serviceNow() - audio.anchor)
          : audio.positionMs) / 1000;
      if (Math.abs(element.currentTime - target) > OPERATOR_SEEK_S) element.currentTime = target;
    }
    playedMs = element.readyState > 0 ? Math.round(element.currentTime * 1000) : null;

    element.volume = fadedVolume();
    if (audio.playing && element.paused) void element.play().catch(() => {});
    if (!audio.playing && !element.paused) element.pause();

    // A fade-out ends in a pause.
    const fade = audio.fade;
    if (
      fade &&
      finishedFade !== fade.startedAt &&
      serviceNow() >= fade.startedAt + fade.durationMs
    ) {
      finishedFade = fade.startedAt;
      engine.dispatch({ type: "pauseAudio" });
    }
  }

  /** The volume, lowered smoothly during a fade-out. */
  function fadedVolume(): number {
    if (!audio) return 1;
    if (!audio.fade) return audio.volume;
    const done = (serviceNow() - audio.fade.startedAt) / audio.fade.durationMs;
    return audio.volume * Math.max(0, 1 - done);
  }

  function seek(event: Event & { currentTarget: HTMLInputElement }) {
    engine.dispatch({ type: "seekAudio", positionMs: event.currentTarget.valueAsNumber });
  }
</script>

<!-- biome-ignore lint/a11y/useMediaCaption: background music, nothing to caption; the element is never shown -->
<audio bind:this={element} onended={() => engine.dispatch({ type: "trackEnded" })}></audio>

{#if audio && track}
  <fieldset class="player" aria-label={t("music.player")}>
    <button
      type="button"
      aria-label={t("music.previous")}
      onclick={() => engine.dispatch({ type: "skipTrack", forward: false })}
    >
      ⏮
    </button>
    <button
      type="button"
      class="play"
      aria-label={audio.playing ? t("common.pause") : t("common.play")}
      onclick={() => engine.dispatch({ type: audio.playing ? "pauseAudio" : "playAudio" })}
    >
      {audio.playing ? "❚❚" : "▶"}
    </button>
    <button
      type="button"
      aria-label={t("music.next")}
      onclick={() => engine.dispatch({ type: "skipTrack", forward: true })}
    >
      ⏭
    </button>

    <span class="track" title={track.name}>♪ {track.name}</span>
    <span class="time">{formatDuration(position)}</span>
    <input
      class="seek"
      type="range"
      min="0"
      max={track.durationMs ?? Math.max(position, 1)}
      step="500"
      value={position}
      aria-label={t("video.position")}
      onchange={seek}
    >
    <input
      class="volume"
      type="range"
      min="0"
      max="1"
      step="0.05"
      value={audio.volume}
      aria-label={t("music.volume")}
      oninput={(e) => engine.dispatch({ type: "setVolume", volume: e.currentTarget.valueAsNumber })}
    >
    <label class="option">
      <input
        type="checkbox"
        checked={audio.loop}
        onchange={(e) => engine.dispatch({ type: "setAudioLoop", on: e.currentTarget.checked })}
      >
      {t("common.loop")}
    </label>
    <button
      type="button"
      class="fade"
      disabled={!audio.playing || audio.fade !== null}
      onclick={() => engine.dispatch({ type: "fadeOutAudio", durationMs: 4000 })}
    >
      {t("music.fadeOut")}
    </button>
  </fieldset>
{/if}

<style>
  .player {
    margin: 0;
    border: none;
    border-top: 1px solid var(--sw-border);
    padding: var(--sw-space-1) var(--sw-space-3);
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    background: var(--sw-surface);
    min-width: 0;
  }

  button {
    min-width: 30px;
    height: 26px;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: var(--sw-surface-raised);
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .fade {
    padding: 0 var(--sw-space-2);
  }

  .track {
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time {
    color: var(--sw-text-muted);
    font-variant-numeric: tabular-nums;
  }

  .seek {
    flex: 1;
  }

  .volume {
    width: 90px;
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }
</style>
