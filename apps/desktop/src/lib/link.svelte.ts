/**
 * Joining another SimpleWorship computer (the one running the service):
 * whether this computer has, as what (an operator or a mirror), and the
 * connection. The link itself is in Rust (see `src-tauri/src/link.rs`).
 *
 * Every window loads the status before it draws anything ({@link load} in
 * `main.ts`): a joined computer takes its pictures and videos from the
 * other computer, sends library questions there (`api.ts`), and keeps its
 * clock in step with it. Joining or leaving reloads every window.
 */

import type { FoundHost, LinkStatus, TakeOverResult } from "@simpleworship/protocol";
import { setClockOffset } from "@simpleworship/renderer";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Must match `STATUS_EVENT` and `CHANGED_EVENT` in `src-tauri/src/link.rs`. */
const STATUS_EVENT = "link://status";
const CHANGED_EVENT = "link://changed";

const NOT_JOINED: LinkStatus = {
  joined: false,
  hostName: "",
  address: "",
  token: "",
  role: null,
  connected: false,
  clockOffsetMs: 0,
  removed: false,
  hostLost: false,
};

class LinkStore {
  status: LinkStatus = $state.raw(NOT_JOINED);
  /** Whether the "Join Another Computer" window is open. */
  joining = $state(false);

  /** This computer runs another computer's service. */
  get joined(): boolean {
    return this.status.joined;
  }

  /** It follows the service but controls nothing. */
  get mirror(): boolean {
    return this.status.joined && this.status.role === "mirror";
  }

  /** Loads the status. Called once per window, before it draws. */
  async load(): Promise<void> {
    this.#apply(await invoke<LinkStatus>("link_status"));
  }

  /** Follows the link; every window reloads when this computer joins or leaves. */
  async connect(): Promise<() => void> {
    const stopStatus = await listen<LinkStatus>(STATUS_EVENT, ({ payload }) =>
      this.#apply(payload),
    );
    const stopChanged = await listen(CHANGED_EVENT, () => window.location.reload());
    return () => {
      stopStatus();
      stopChanged();
    };
  }

  /** Looks for computers to join on the network (takes a few seconds). */
  discover(): Promise<FoundHost[]> {
    return invoke<FoundHost[]>("link_discover");
  }

  /** Joins the computer at `address`; resolves once its operator allowed it. */
  join(address: string, code: string, name: string): Promise<LinkStatus> {
    return invoke<LinkStatus>("link_join", { address, code, name });
  }

  leave(): Promise<LinkStatus> {
    return invoke<LinkStatus>("link_leave");
  }

  /** The main computer stopped answering: carries on its service here. */
  takeOver(): Promise<TakeOverResult> {
    return invoke<TakeOverResult>("link_take_over");
  }

  #apply(status: LinkStatus) {
    this.status = status;
    setClockOffset(status.joined ? status.clockOffsetMs : 0);
  }
}

export const link = new LinkStore();
