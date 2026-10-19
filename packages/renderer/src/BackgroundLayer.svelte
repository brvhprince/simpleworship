<!--
  @component
  Draws a theme background filling the canvas: a colour, a gradient, an
  image or a video.

  - `still`: show a video as its thumbnail instead of playing it (previews
    and slide thumbnails).
  - `playback` and `muted` are passed to live videos (see `VideoLayer`).
-->
<script lang="ts">
  import type { Background, VideoPlayback } from "@simpleworship/protocol";
  import { getMediaUrls } from "./media.ts";
  import { backgroundCss } from "./styles.ts";
  import VideoLayer from "./VideoLayer.svelte";

  interface Props {
    background: Background;
    still?: boolean;
    playback?: VideoPlayback | null;
    muted?: boolean;
  }

  let { background, still = false, playback = null, muted = false }: Props = $props();

  const urls = getMediaUrls();
</script>

<div class="background" style:background={backgroundCss(background)}>
  {#if background.type === "image"}
    <img src={urls.file(background.media)} class={background.fit} alt="">
  {:else if background.type === "video" && still}
    <img src={urls.thumbnail(background.media)} class={background.fit} alt="">
  {:else if background.type === "video"}
    <VideoLayer
      media={background.media}
      fit={background.fit}
      {playback}
      muted={muted || (playback === null && !background.playAudio)}
    />
  {/if}
</div>

<style>
  .background {
    position: absolute;
    inset: 0;
    background: #000;
  }

  img {
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
