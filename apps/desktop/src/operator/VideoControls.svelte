<!--
  @component
  Play, pause, seek, loop and mute for the live video, under the live pane.
  Shown only while a video is live. Every screen follows these controls, because they
  change the engine's shared playback clock.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { formatDuration } from "../lib/format.ts";
  import { media } from "../lib/media.svelte.ts";

  const playback = $derived(engine.output?.video ?? null);
  const duration = $derived(
    media.list.find((item) => item.id === playback?.mediaId)?.durationMs ?? null,
  );

  /** Ticks every quarter second so the position display moves. */
  let now = $state(Date.now());
  onMount(() => {
    const timer = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(timer);
  });

  /** Where the video is now, in milliseconds (mirrors `VideoPlayback::position_at`). */
  const position = $derived.by(() => {
    if (!playback) return 0;
    const elapsed = playback.playing ? Math.max(0, now - playback.anchor) : 0;
    const raw = playback.positionMs + elapsed;
    if (!duration) return raw;
    return playback.loop ? raw % duration : Math.min(raw, duration);
  });

  function seek(event: Event & { currentTarget: HTMLInputElement }) {
    engine.dispatch({ type: "seekVideo", positionMs: event.currentTarget.valueAsNumber });
  }
</script>

{#if playback}
  <fieldset class="video-controls" aria-label="Live video">
    <button
      type="button"
      class="play"
      aria-label={playback.playing ? "Pause" : "Play"}
      onclick={() => engine.dispatch({ type: playback.playing ? "pauseVideo" : "playVideo" })}
    >
      {playback.playing ? "❚❚" : "▶"}
    </button>
    <span class="time">{formatDuration(position)}</span>
    <input
      type="range"
      min="0"
      max={duration ?? Math.max(position, 1)}
      step="100"
      value={position}
      aria-label="Position"
      onchange={seek}
    >
    <span class="time">{duration ? `-${formatDuration(duration - position)}` : ""}</span>
    <label class="option">
      <input
        type="checkbox"
        checked={playback.loop}
        onchange={(e) => engine.dispatch({ type: "setVideoLoop", on: e.currentTarget.checked })}
      >
      Loop
    </label>
    <label class="option" title="Silences the video on the projector and every other screen">
      <input
        type="checkbox"
        checked={playback.muted}
        onchange={(e) => engine.dispatch({ type: "setVideoMuted", on: e.currentTarget.checked })}
      >
      Mute
    </label>
  </fieldset>
{/if}

<style>
  .video-controls {
    border: none;
    padding: 0;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    margin: 0 var(--sw-space-3) var(--sw-space-3);
  }

  .play {
    width: 32px;
    height: 28px;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: var(--sw-surface-raised);
    cursor: pointer;
  }

  input[type="range"] {
    flex: 1;
    accent-color: var(--sw-live);
  }

  .time {
    min-width: 3.5em;
    font-variant-numeric: tabular-nums;
    color: var(--sw-text-muted);
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }
</style>
