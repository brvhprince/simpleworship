<!--
  @component
  Stands for a camera where it is not shown live (thumbnails, phones): a
  camera symbol and the camera's name, on dark grey.
-->
<script lang="ts">
  import type { CameraRef } from "@simpleworship/protocol";

  interface Props {
    camera: CameraRef;
  }

  let { camera }: Props = $props();
</script>

<div class="placeholder">
  <svg viewBox="0 0 24 24" width="320" height="320" aria-hidden="true">
    <path
      fill="currentColor"
      d="M4 6h11a2 2 0 0 1 2 2v1.6l3.4-2.3A1 1 0 0 1 22 8.1v7.8a1 1 0 0 1-1.6.8L17 14.4V16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2Z"
    />
  </svg>
  <span>{camera.label}</span>
</div>

<style>
  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 24px;
    background: #26292f;
    color: #8a8f99;
    font:
      600 64px / 1.2 "Open Sans",
      sans-serif;
  }
</style>
