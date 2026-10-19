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
  import { needsCorrection, targetSeconds } from "./video-sync.ts";

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
  const SYNC_INTERVAL_MS = 1000;

  const urls = getMediaUrls();
  let video: HTMLVideoElement;

  /** Silent if this copy is muted, or the operator muted the video. */
  const silent = $derived(muted || (playback?.muted ?? false));

  /** Brings the element in line with the shared playback state. */
  function sync() {
    if (!video || !playback) return;
    const target = targetSeconds(playback, Date.now(), video.duration);
    if (needsCorrection(video.currentTime, target)) {
      video.currentTime = target;
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
