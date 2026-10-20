<!--
  @component
  A camera's live picture, filling its parent (see `camera.ts`). Silent:
  cameras are shown without their sound. If the camera cannot be opened,
  the layer stays black, with a note where the window explains problems
  (the operator's panes, never the projector).
-->
<script lang="ts">
  import type { CameraRef, MediaFit } from "@simpleworship/protocol";
  import { getCameraUse, useCamera } from "./camera.ts";

  interface Props {
    camera: CameraRef;
    fit: MediaFit;
  }

  let { camera, fit }: Props = $props();

  const use = getCameraUse();

  let video: HTMLVideoElement;
  let problem: string | null = $state(null);

  // Reopen when the camera changes; share it with other layers.
  $effect(() => {
    const handle = useCamera({ deviceId: camera.deviceId, label: camera.label });
    problem = null;
    let current = true;
    handle.stream.then(
      (stream) => {
        if (!current || !video) return;
        video.srcObject = stream;
        void video.play().catch(() => {});
      },
      (error: Error) => {
        if (current) problem = error.message;
      },
    );
    return () => {
      current = false;
      if (video) video.srcObject = null;
      handle.release();
    };
  });
</script>

<!-- svelte-ignore a11y_media_has_caption: a live camera has no captions -->
<video bind:this={video} class={fit} autoplay muted playsinline></video>
{#if problem && use.explain}
  <p class="problem">{camera.label}: {problem}</p>
{/if}

<style>
  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }

  .cover {
    object-fit: cover;
  }

  .contain {
    object-fit: contain;
  }

  .stretch {
    object-fit: fill;
  }

  .problem {
    position: absolute;
    left: 0;
    right: 0;
    top: 45%;
    margin: 0;
    color: #888;
    font:
      600 40px / 1.3 "Open Sans",
      sans-serif;
    text-align: center;
  }
</style>
