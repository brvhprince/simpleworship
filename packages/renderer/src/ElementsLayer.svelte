<!--
  @component
  Draws a slide made in the canvas editor: its text boxes, shapes and
  pictures, back to front, each where the operator put it. Pictures and
  videos have nothing behind them, so transparent ones (animated WebP or
  PNG, or a video with an alpha channel) show the slide through.

  - `hideText`: leave out the text boxes (the operator's Clear Text).
  - `still`: show videos paused on their first frame (previews and
    thumbnails), instead of playing them.
  - `muted`: silence videos (every screen except the projector).
-->
<script lang="ts">
  import type { SlideElement } from "@simpleworship/protocol";
  import { autofit } from "./autofit.ts";
  import {
    frameStyle,
    lineStyle,
    objectFit,
    runStyle,
    shapeStyle,
    textBoxStyle,
    verticalJustify,
  } from "./elements.ts";
  import { getMediaUrls } from "./media.ts";
  import { textStyle } from "./styles.ts";

  interface Props {
    elements: SlideElement[];
    hideText?: boolean;
    still?: boolean;
    muted?: boolean;
  }

  let { elements, hideText = false, still = false, muted = false }: Props = $props();

  const urls = getMediaUrls();
</script>

{#each elements as element (element.id)}
  {@const content = element.content}
  {#if content.type !== "text" || !hideText}
    <div class="element" style={frameStyle(element.frame, element.opacity)}>
      {#if content.type === "text"}
        <div class="box" style={textBoxStyle(content)}>
          <div
            class="fit"
            style:justify-content={verticalJustify(content)}
            use:autofit={{
              enabled: content.autoFit,
              maxSize: content.style.fontSize,
              scaleUp: true,
              content: [content.runs, content.style, element.frame, content.padding],
            }}
          >
            <div class="text" style={textStyle(content.style)}>
              {#each content.runs as run, index (index)}
                <span style={runStyle(run, content.style)}>{run.text}</span>
              {/each}
            </div>
          </div>
        </div>
      {:else if content.type === "shape"}
        {#if content.shape === "line"}
          <span style={lineStyle(content, element.frame)}></span>
        {:else}
          <div class="shape" style={shapeStyle(content)}></div>
        {/if}
      {:else if content.kind === "video"}
        <!-- A media fragment shows the first frame when not playing. -->
        <video
          src={still ? `${urls.file(content.media)}#t=0.1` : urls.file(content.media)}
          style:object-fit={objectFit(content.fit)}
          style:border-radius={content.cornerRadius ? `${content.cornerRadius}px` : null}
          autoplay={!still}
          loop
          playsinline
          preload="auto"
          muted={still || muted || !content.playAudio}
        ></video>
      {:else}
        <img
          src={urls.file(content.media)}
          alt=""
          style:object-fit={objectFit(content.fit)}
          style:border-radius={content.cornerRadius ? `${content.cornerRadius}px` : null}
        >
      {/if}
    </div>
  {/if}
{/each}

<style>
  .element {
    position: absolute;
  }

  .box {
    position: absolute;
    inset: 0;
    box-sizing: border-box;
    overflow: hidden;
  }

  .fit {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .text {
    /* Keep the typed line breaks; wrap long lines at spaces only. */
    white-space: pre-wrap;
    overflow-wrap: normal;
  }

  .shape {
    position: absolute;
    inset: 0;
  }

  /* No background: transparent pictures and videos show the slide through. */
  img,
  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    background: transparent;
  }
</style>
