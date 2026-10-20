/**
 * Which toolbar panel (Alerts, Stage) is open. Shared, so a keyboard
 * shortcut can open a panel as well as its button.
 */
class Panels {
  /** The open panel, if any. */
  open: "alerts" | "stage" | "transitions" | "remote" | "stream" | "screens" | null = $state(null);

  toggle(panel: "alerts" | "stage" | "transitions" | "remote" | "stream" | "screens"): void {
    this.open = this.open === panel ? null : panel;
  }

  close(): void {
    this.open = null;
  }
}

export const panels = new Panels();
