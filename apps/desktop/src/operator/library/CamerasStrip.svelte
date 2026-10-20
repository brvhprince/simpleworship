<!--
  @component
  The Cameras section of the Media tab: cameras and capture cards connected
  to this computer. Nothing is opened just by looking: each camera shows
  its symbol and name until it is clicked, and then only that one shows
  its live picture (opening a camera lights it up, and wakes a phone used
  as a camera). Double-click (or "Show in Preview") to preview one full
  screen, "Add to Schedule" to add it.

  The first time, "Find Cameras" asks for permission (macOS asks once);
  after that the list fills in by itself and follows cameras being plugged
  in and out.
-->
<script lang="ts">
  import type { CameraRef } from "@simpleworship/protocol";
  import {
    BackgroundLayer,
    listCameras,
    onCamerasChanged,
    ScaledCanvas,
  } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { addCameraToSchedule } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";

  /** Remembered on this computer once cameras have been found. */
  const FOUND_KEY = "simpleworship.camerasFound";

  let cameras: CameraRef[] = $state([]);
  /** The camera clicked: the only one shown live here. */
  let selectedId: string | null = $state(null);
  let searched = $state(false);
  let problem: string | null = $state(null);

  async function find() {
    try {
      cameras = await listCameras();
      searched = true;
      problem = null;
      try {
        localStorage.setItem(FOUND_KEY, "1");
      } catch {
        // Not remembered.
      }
    } catch (error) {
      problem = String(error);
    }
  }

  function preview(camera: CameraRef) {
    void engine.dispatch({ type: "previewCamera", camera });
  }

  function add(camera: CameraRef) {
    addCameraToSchedule(camera).catch((error) => engine.showError(error));
  }

  onMount(() => {
    let found = false;
    try {
      found = localStorage.getItem(FOUND_KEY) === "1";
    } catch {
      // Not remembered.
    }
    if (found) void find();
    return onCamerasChanged(() => {
      if (searched) void find();
    });
  });
</script>

<section class="cameras" aria-label={t("cameras.title")}>
  <header>
    <h3>{t("cameras.title")}</h3>
    <Button onclick={find}>{searched ? t("cameras.refresh") : t("cameras.find")}</Button>
    {#if problem}
      <span class="problem">{t("cameras.notAllowed")}</span>
    {:else if !searched}
      <span class="hint">{t("cameras.hint")}</span>
    {:else if cameras.length === 0}
      <span class="hint">{t("cameras.none")}</span>
    {:else}
      <span class="hint">{t("cameras.clickToSee")}</span>
    {/if}
  </header>
  {#if cameras.length > 0}
    <ul>
      {#each cameras as camera (camera.deviceId)}
        {@const selected = camera.deviceId === selectedId}
        <li>
          <button
            type="button"
            class="picture"
            class:selected
            aria-pressed={selected}
            onclick={() => (selectedId = selected ? null : camera.deviceId)}
            ondblclick={() => preview(camera)}
          >
            <ScaledCanvas>
              <BackgroundLayer
                background={{ type: "camera", camera, fit: "cover", opacity: 1 }}
                still={!selected}
              />
            </ScaledCanvas>
          </button>
          <span class="name" title={camera.label}>{camera.label}</span>
          {#if selected}
            <span class="actions">
              <Button onclick={() => add(camera)}>{t("common.addToSchedule")}</Button>
              <Button variant="primary" onclick={() => preview(camera)}>
                {t("common.showInPreview")}
              </Button>
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .cameras {
    padding: var(--sw-space-2) var(--sw-space-3);
    border-top: 1px solid var(--sw-border);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
  }

  h3 {
    margin: 0;
    font-size: var(--sw-font-size);
  }

  .hint {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .problem {
    color: var(--sw-warning, #f59f00);
    font-size: var(--sw-font-size-small);
  }

  ul {
    list-style: none;
    margin: var(--sw-space-2) 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--sw-space-3);
  }

  li {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
  }

  .picture {
    padding: 0;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    overflow: hidden;
    cursor: pointer;
    background: #000;
  }

  .picture.selected {
    border-color: var(--sw-accent);
    box-shadow: 0 0 0 1px var(--sw-accent);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: var(--sw-space-1);
  }
</style>
