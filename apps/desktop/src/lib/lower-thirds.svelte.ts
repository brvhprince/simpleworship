/**
 * The operator's lower thirds: the saved list, the one being edited, and
 * showing or hiding one (see `crates/sw-core/src/lower_thirds.rs`). Shared
 * by the toolbar's Lower Third panel and the editor dialog, which lives in
 * the operator window so it stays open when the panel closes.
 */

import type { LowerThird, LowerThirdKind } from "@simpleworship/protocol";
import { getLowerThirds, saveLowerThirds, uuidV7 } from "./api.ts";
import { engine } from "./engine.svelte.ts";

/** The usual look: a blue accent, white text on a dark box. Matches `LowerThirdLook::default`. */
const DEFAULT_LOOK: LowerThird["look"] = {
  style: "bar",
  accent: "#4c6ef5",
  background: "rgba(10, 12, 18, 0.86)",
  color: "#ffffff",
  fontFamily: "",
  side: "left",
  icon: "none",
  rounding: 0,
  size: "medium",
  speed: "normal",
};

class LowerThirdsStore {
  /** The saved lower thirds, in order. */
  list: LowerThird[] = $state([]);
  /** The lower third open in the editor (a copy), if any. */
  editing: LowerThird | null = $state(null);
  /** Whether `editing` is not in the list yet. */
  isNew = $state(false);
  /**
   * Whether the saved list has arrived. Until it has, nothing can be added,
   * changed or deleted: saving a list built on an empty one would replace
   * the real list (on a slow connection to the main computer, or after a
   * failed load).
   */
  ready = $state(false);
  /** Counts saves, so a load that started before one does not undo it. */
  #saves = 0;
  /** Counts loads, so only the newest one's answer is used (answers can arrive out of order). */
  #loads = 0;
  /**
   * Whether the list is being written. Saves and deletes each write the
   * whole list, so one at a time: a late save could otherwise bring back
   * an item just deleted.
   */
  busy = $state(false);

  /** Loads the saved list (the main computer's, on one that joined). */
  async load(): Promise<void> {
    const load = ++this.#loads;
    const savesBefore = this.#saves;
    try {
      const list = await getLowerThirds();
      // Only the newest load counts, and not if a save came since: the
      // list then already holds something newer than this answer.
      if (load !== this.#loads || this.#saves !== savesBefore) return;
      this.list = list;
      this.ready = true;
    } catch (error) {
      if (load === this.#loads) engine.showError(error);
    }
  }

  /** Shows `item` on the outputs it is for, replacing any showing. */
  show(item: LowerThird): void {
    engine.dispatch({ type: "showLowerThird", item: $state.snapshot(item) as LowerThird });
  }

  /** Takes the lower third off; it animates out. */
  hide(): void {
    engine.dispatch({ type: "hideLowerThird" });
  }

  /** Opens a new lower third of `kind` in the editor, with the look of the last one. */
  create(kind: LowerThirdKind = "name"): void {
    if (!this.ready || this.busy) return;
    const last = this.list.at(-1);
    this.editing = {
      id: uuidV7(),
      kind,
      title: "",
      subtitle: "",
      look: structuredClone(last ? $state.snapshot(last.look) : DEFAULT_LOOK),
      target: "both",
      hideAfterSeconds: null,
    };
    this.isNew = true;
  }

  /** Opens a copy of `item` in the editor. */
  edit(item: LowerThird): void {
    if (!this.ready || this.busy) return;
    this.editing = structuredClone($state.snapshot(item)) as LowerThird;
    this.isNew = false;
  }

  /** Saves the edited lower third into the list. Resolves to it, or `null` if saving failed. */
  async saveEditing(): Promise<LowerThird | null> {
    const item = this.editing;
    if (!item) return null;
    const saved = structuredClone($state.snapshot(item)) as LowerThird;
    const list = this.isNew
      ? [...this.list, saved]
      : this.list.map((other) => (other.id === saved.id ? saved : other));
    if (await this.#store(list)) {
      // Close the editor only if it still holds what was saved (Cancel
      // stays enabled during a save, and another item may be open now).
      if (this.editing?.id === saved.id) this.editing = null;
      return saved;
    }
    return null;
  }

  /** Deletes the edited lower third. */
  async deleteEditing(): Promise<void> {
    const item = this.editing;
    if (!item) return;
    const removed = await this.#store(this.list.filter((other) => other.id !== item.id));
    if (removed && this.editing?.id === item.id) this.editing = null;
  }

  async #store(list: LowerThird[]): Promise<boolean> {
    if (!this.ready || this.busy) return false;
    this.busy = true;
    try {
      await saveLowerThirds($state.snapshot(list) as LowerThird[]);
      this.#saves += 1;
      this.list = list;
      return true;
    } catch (error) {
      engine.showError(error);
      return false;
    } finally {
      this.busy = false;
    }
  }
}

export const lowerThirds = new LowerThirdsStore();
