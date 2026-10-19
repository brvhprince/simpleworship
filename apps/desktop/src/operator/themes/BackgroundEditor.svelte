<!--
  @component
  Edits a background: a colour, a two-colour gradient, an image or a video
  from the media library.
-->
<script lang="ts">
  import type { Background, MediaFit, MediaItem } from "@simpleworship/protocol";
  import ColorField from "./ColorField.svelte";
  import MediaPicker from "./MediaPicker.svelte";

  interface Props {
    value: Background;
    onchange: (background: Background) => void;
  }

  let { value, onchange }: Props = $props();

  /** "image"/"video" chosen in the select before a file is picked. */
  let pendingType: "image" | "video" | null = $state(null);
  const shownType = $derived(pendingType ?? value.type);

  /** Switching type keeps what can be kept (colours, the chosen media). */
  function setType(type: Background["type"]) {
    switch (type) {
      case "color":
        onchange({ type, color: value.type === "linearGradient" ? value.from : "#000000" });
        break;
      case "linearGradient":
        onchange({
          type,
          from: value.type === "color" ? value.color : "#0b1a33",
          to: "#000000",
          angleDeg: 180,
        });
        break;
      case "image":
      case "video":
        // Nothing to show until media is picked; keep the old background.
        pendingType = type;
        break;
    }
  }

  function pick(media: MediaItem) {
    pendingType = null;
    const reference = { id: media.id, file: media.file };
    const fit: MediaFit = "cover";
    if (media.kind === "video") {
      onchange({ type: "video", media: reference, fit, loop: true, playAudio: false });
    } else {
      onchange({ type: "image", media: reference, fit });
    }
  }
</script>

<div class="background-editor">
  <select
    value={shownType}
    aria-label="Background type"
    onchange={(e) => setType(e.currentTarget.value as Background["type"])}
  >
    <option value="color">Colour</option>
    <option value="linearGradient">Gradient</option>
    <option value="image">Image</option>
    <option value="video">Video</option>
  </select>

  {#if shownType === "color" && value.type === "color"}
    <ColorField
      label="Colour"
      value={value.color}
      onchange={(color) => onchange({ ...value, color })}
    />
  {:else if shownType === "linearGradient" && value.type === "linearGradient"}
    <ColorField label="From" value={value.from} onchange={(from) => onchange({ ...value, from })} />
    <ColorField label="To" value={value.to} onchange={(to) => onchange({ ...value, to })} />
    <label class="inline">
      Angle
      <input
        type="number"
        min="0"
        max="360"
        value={value.angleDeg}
        onchange={(e) => onchange({ ...value, angleDeg: e.currentTarget.valueAsNumber || 0 })}
      >°
    </label>
  {:else if shownType === "image" || shownType === "video"}
    <MediaPicker
      kind={shownType}
      value={value.type === "image" || value.type === "video" ? value.media : null}
      onchange={pick}
    />
    {#if value.type === "image" || value.type === "video"}
      <label class="inline">
        Fit
        <select
          value={value.fit}
          onchange={(e) => onchange({ ...value, fit: e.currentTarget.value as MediaFit })}
        >
          <option value="cover">Fill the screen (crop edges)</option>
          <option value="contain">Show it all (black bars)</option>
        </select>
      </label>
    {/if}
    {#if value.type === "video"}
      <label class="inline">
        <input
          type="checkbox"
          checked={value.loop}
          onchange={(e) => onchange({ ...value, loop: e.currentTarget.checked })}
        >
        Loop
      </label>
      <label class="inline">
        <input
          type="checkbox"
          checked={value.playAudio}
          onchange={(e) => onchange({ ...value, playAudio: e.currentTarget.checked })}
        >
        Play sound
      </label>
    {/if}
  {/if}
</div>

<style>
  .background-editor {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .inline {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  input[type="number"] {
    width: 64px;
  }
</style>
