/**
 * Per-media aspect ratio scaling preference:
 * - "automatic": follow context (cover for background, contain for standalone)
 * - "stretch": fill screen, distorting aspect ratio if needed (object-fit: fill)
 * - "maintain": maintain aspect ratio with letterbox/pillarbox (object-fit: contain)
 * - "zoom": crop to fill entire frame (object-fit: cover)
 */
export type MediaFitPreference = "automatic" | "stretch" | "maintain" | "zoom";

const STORAGE_KEY = "simpleworship.media_fit";

function loadPreferences(): Record<string, MediaFitPreference> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

export function getMediaFitPreference(mediaId: string): MediaFitPreference {
  const prefs = loadPreferences();
  return prefs[mediaId] ?? "automatic";
}

export function setMediaFitPreference(mediaId: string, fit: MediaFitPreference): void {
  try {
    const prefs = loadPreferences();
    if (fit === "automatic") {
      delete prefs[mediaId];
    } else {
      prefs[mediaId] = fit;
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent("media-fit-changed", { detail: { mediaId, fit } }));
    }
  } catch {
    // Ignore storage errors
  }
}

export function resolveFitClass(
  mediaId: string | undefined | null,
  fallback: "cover" | "contain" | "stretch",
): "cover" | "contain" | "stretch" {
  if (!mediaId) return fallback;
  const pref = getMediaFitPreference(mediaId);
  if (pref === "stretch") return "stretch";
  if (pref === "maintain") return "contain";
  if (pref === "zoom") return "cover";
  return fallback;
}
