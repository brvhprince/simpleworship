<!--
  @component
  Draws a theme background filling the canvas: a colour, a gradient, an
  image, a video or a camera.

  - `still`: show a video as its thumbnail instead of playing it, and a
    camera as a symbol (previews and slide thumbnails).
  - `playback` and `muted` are passed to live videos (see `VideoLayer`).
-->
<script lang="ts">
  import type { Background, VideoPlayback } from "@simpleworship/protocol";
  import CameraLayer from "./CameraLayer.svelte";
  import CameraPlaceholder from "./CameraPlaceholder.svelte";
  import { getCameraUse } from "./camera.ts";
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
  const cameras = getCameraUse();

  /** Pictures and videos can be faded into the black behind them. */
  const opacity = $derived(
    background.type === "image" || background.type === "video" || background.type === "camera"
      ? background.opacity
      : 1,
  );
</script>

<div class="background" style:background={backgroundCss(background)}>
  <div class="media" style:opacity={opacity < 1 ? opacity : null}>
    {#if background.type === "image"}
      <img src={urls.file(background.media)} class={background.fit} alt="">
    {:else if background.type === "video" && still}
      <img src={urls.thumbnail(background.media)} class={background.fit} alt="">
    {:else if background.type === "camera" && cameras.live && !still}
      <CameraLayer camera={background.camera} fit={background.fit} />
    {:else if background.type === "camera" && cameras.placeholder}
      <CameraPlaceholder camera={background.camera} />
    {:else if background.type === "video"}
      <VideoLayer
        media={background.media}
        fit={background.fit}
        {playback}
        muted={muted || (playback === null && !background.playAudio)}
      />
    {/if}
  </div>
</div>

<style>
  .background {
    position: absolute;
    inset: 0;
    background: #000;
  }

  .media {
    position: absolute;
    inset: 0;
  }

  img {
    width: 100%;
    height: 100%;
  }

  .stretch {
    object-fit: fill;
  }

  .cover {
    object-fit: cover;
  }

  .contain {
    object-fit: contain;
  }
</style>
