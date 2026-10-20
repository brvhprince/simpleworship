<!--
  @component
  A video that follows the engine's shared playback clock (see
  `video-sync.ts`). Used for live output; previews and thumbnails show the
  still thumbnail instead, so a schedule full of videos does not try to
  play them all at once.
-->
<script lang="ts">
  import type { MediaFit, MediaRef, VideoPlayback } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { getMediaUrls } from "./media.ts";
  import { now as serviceNow } from "./timing.ts";
  import { correctionFor, driftSeconds, isBuffering, targetSeconds } from "./video-sync.ts";

  interface Props {
    media: MediaRef;
    fit: MediaFit;
    /** The shared playback state, or `null` to just play from the start. */
    playback: VideoPlayback | null;
    /** Silence this copy whatever the playback says (the operator's live
     *  pane always is, so only the projector plays sound). */
    muted: boolean;
  }

  let { media, fit, playback, muted }: Props = $props();

  /** How often to check for drift, in milliseconds. */
  const SYNC_INTERVAL_MS = 500;

  /** After a seek, give the player this long to settle before seeking again. */
  const SEEK_COOLDOWN_MS = 3000;

  /** How far off the operator's own changes (a seek) may land, in seconds. */
  const OPERATOR_SEEK_S = 0.25;

  const urls = getMediaUrls();
  let video: HTMLVideoElement;
  let lastSeekAt = 0;
  /**
   * The timeline last acted on, to tell the operator's changes (seek, play,
   * pause) from drift. Compared by value: every engine event brings a new
   * copy of the same playback, e.g. when the lyrics change.
   */
  let handled = "";

  /** Silent if this copy is muted, or the operator muted the video. */
  const silent = $derived(muted || (playback?.muted ?? false));

  /**
   * Brings the element in line with the shared playback state: straight
   * away when the operator changed something (seek, play, pause), gently
   * for drift (see `video-sync.ts`).
   */
  function sync() {
    if (!video || !playback) return;
    const now = serviceNow();
    const target = targetSeconds(playback, now, video.duration);
    const drift = driftSeconds(video.currentTime, target, video.duration, playback.loop);
    const timeline = `${playback.positionMs}@${playback.anchor}:${playback.playing}`;
    const changed = handled !== timeline;
    handled = timeline;

    if (changed && Math.abs(drift) > OPERATOR_SEEK_S) {
      seekTo(target, now);
    } else if (!isBuffering(video) && playback.playing) {
      const correction = correctionFor(drift);
      if (correction.seek) {
        if (now - lastSeekAt > SEEK_COOLDOWN_MS) seekTo(target, now);
      } else {
        video.playbackRate = correction.rate;
      }
    }
    video.loop = playback.loop;
    video.muted = silent;

    if (playback.playing && video.paused && !video.ended) {
      // A webview may refuse to start a video with sound on its own; play
      // it muted rather than show a frozen frame.
      video.play().catch(() => {
        video.muted = true;
        void video.play();
      });
    } else if (!playback.playing && !video.paused) {
      video.pause();
    }
  }

  function seekTo(seconds: number, now: number) {
    video.currentTime = seconds;
    video.playbackRate = 1;
    lastSeekAt = now;
  }

  // Re-sync whenever the engine changes the playback (play, pause, seek,
  // loop, mute).
  $effect(() => {
    void playback;
    void silent;
    sync();
  });

  onMount(() => {
    const timer = setInterval(sync, SYNC_INTERVAL_MS);
    return () => clearInterval(timer);
  });
</script>

<video
  bind:this={video}
  src={urls.file(media)}
  class={fit}
  muted={silent}
  playsinline
  autoplay={playback?.playing ?? true}
  onloadedmetadata={sync}
></video>

<style>
  video {
    width: 100%;
    height: 100%;
  }

  .cover {
    object-fit: cover;
  }

  .contain {
    object-fit: contain;
  }
</style>
