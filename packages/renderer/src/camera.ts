/**
 * Live pictures from cameras and capture cards (a `CameraRef`).
 *
 * A window opens each camera once, however many layers show it, and closes
 * it a few seconds after the last one goes, so going from slide to slide
 * over the same camera never blinks.
 *
 * Each host says, with {@link setCameraUse}, what its windows do with
 * cameras: the projector and the operator's panes show them live; phones,
 * browser sources and NDI pages never open a camera (it is not theirs, or
 * cannot be sent smoothly).
 */

import type { CameraRef } from "@simpleworship/protocol";
import { getContext, setContext } from "svelte";

/** What a window does with camera backgrounds. */
export interface CameraUse {
  /** Open cameras and show them live. */
  live: boolean;
  /** Where a camera is not shown live (thumbnails, or `live` off), show a
   *  camera symbol and its name instead of nothing. */
  placeholder: boolean;
  /** Say so on screen when a camera cannot be opened (the operator's
   *  panes; never the projector). */
  explain: boolean;
}

const CAMERA_USE = Symbol("simpleworship.cameraUse");

/** Sets what renderer components below the caller do with cameras. */
export function setCameraUse(use: CameraUse): void {
  setContext(CAMERA_USE, use);
}

/** What this window does with cameras. Without a host: placeholders only. */
export function getCameraUse(): CameraUse {
  return (
    getContext<CameraUse | undefined>(CAMERA_USE) ?? {
      live: false,
      placeholder: true,
      explain: false,
    }
  );
}

/** How long a camera stays open after the last layer showing it goes. */
const CLOSE_AFTER_MS = 5000;

interface Open {
  stream: Promise<MediaStream>;
  users: number;
  closing?: ReturnType<typeof setTimeout>;
}

const open = new Map<string, Open>();

function keyOf(camera: CameraRef): string {
  return `${camera.deviceId}\u0000${camera.label}`;
}

/**
 * Opens `camera` (or shares it if it is already open). Call `release` when
 * the picture is no longer shown.
 */
export function useCamera(camera: CameraRef): { stream: Promise<MediaStream>; release(): void } {
  const key = keyOf(camera);
  let entry = open.get(key);
  if (entry) {
    clearTimeout(entry.closing);
    entry.users += 1;
  } else {
    entry = { stream: openStream(camera), users: 1 };
    open.set(key, entry);
    // A camera that failed to open is tried again next time.
    entry.stream.catch(() => open.delete(key));
  }
  const used = entry;
  let released = false;
  return {
    stream: used.stream,
    release() {
      if (released) return;
      released = true;
      used.users -= 1;
      if (used.users > 0) return;
      used.closing = setTimeout(() => {
        if (open.get(key) !== used || used.users > 0) return;
        open.delete(key);
        void used.stream.then((stream) => {
          for (const track of stream.getTracks()) track.stop();
        });
      }, CLOSE_AFTER_MS);
    },
  };
}

/** The cameras connected now, asking for permission first if needed. */
export async function listCameras(): Promise<CameraRef[]> {
  let devices = await videoInputs();
  // Until a camera has been allowed, the system hides their names.
  if (devices.length > 0 && devices.every((device) => !device.label)) {
    const stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: false });
    for (const track of stream.getTracks()) track.stop();
    devices = await videoInputs();
  }
  return devices.map((device) => ({ deviceId: device.deviceId, label: device.label }));
}

/** Calls `onchange` when a camera is plugged in or out. Returns a function that stops. */
export function onCamerasChanged(onchange: () => void): () => void {
  navigator.mediaDevices?.addEventListener("devicechange", onchange);
  return () => navigator.mediaDevices?.removeEventListener("devicechange", onchange);
}

async function videoInputs(): Promise<MediaDeviceInfo[]> {
  if (!navigator.mediaDevices?.enumerateDevices) return [];
  const devices = await navigator.mediaDevices.enumerateDevices();
  return devices.filter((device) => device.kind === "videoinput");
}

/** Finds the camera by its id, else by its name, and opens it at 1080p. */
async function openStream(camera: CameraRef): Promise<MediaStream> {
  let devices = await videoInputs();
  if (devices.every((device) => !device.deviceId)) {
    // No permission yet: ask, then look again.
    await listCameras();
    devices = await videoInputs();
  }
  const device =
    devices.find((device) => device.deviceId === camera.deviceId) ??
    devices.find((device) => device.label === camera.label);
  if (!device) throw new Error(`not connected: ${camera.label}`);
  return navigator.mediaDevices.getUserMedia({
    audio: false,
    video: {
      deviceId: { exact: device.deviceId },
      width: { ideal: 1920 },
      height: { ideal: 1080 },
      frameRate: { ideal: 30 },
    },
  });
}
