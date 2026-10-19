<!--
  @component
  Draws a theme background (colour, gradient, image or video) filling the canvas.
-->
<script lang="ts">
  import type { Background } from "@simpleworship/protocol";
  import { getMediaResolver } from "./media.ts";
  import { backgroundCss } from "./styles.ts";

  interface Props {
    background: Background;
  }

  let { background }: Props = $props();

  const resolveMedia = getMediaResolver();
</script>

<div class="background" style:background={backgroundCss(background)}>
  {#if background.type === "image"}
    <img src={resolveMedia(background.path)} alt="">
  {:else if background.type === "video"}
    <video
      src={resolveMedia(background.path)}
      autoplay
      playsinline
      loop={background.loop}
      muted={!background.playAudio}
    ></video>
  {/if}
</div>

<style>
  .background {
    position: absolute;
    inset: 0;
  }

  img,
  video {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
