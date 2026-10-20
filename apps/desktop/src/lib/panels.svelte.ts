/**
 * Which toolbar panel (Alerts, Stage, Listen…) is open. Shared, so a
 * keyboard shortcut can open a panel as well as its button.
 */

/** The toolbar's panels. */
export type Panel = "alerts" | "stage" | "transitions" | "remote" | "stream" | "screens" | "listen";

class Panels {
  /** The open panel, if any. */
  open: Panel | null = $state(null);

  toggle(panel: Panel): void {
    this.open = this.open === panel ? null : panel;
  }

  close(): void {
    this.open = null;
  }
}

export const panels = new Panels();
