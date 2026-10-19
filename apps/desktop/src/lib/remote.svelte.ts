/**
 * The phone remote, from the operator's side: whether it is on, which
 * phones are paired and connected, and phones asking to pair (which the
 * operator allows, choosing a role, or refuses).
 *
 * The server itself runs in Rust (see `src-tauri/src/remote.rs`).
 */

import type { PairingCode, PairRequest, RemoteStatus, Role } from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { engine } from "./engine.svelte.ts";
import type { MessageKey } from "./i18n/i18n.svelte.ts";

/** Must match the events in `src-tauri/src/remote.rs`. */
const STATUS_EVENT = "remote://status";
const PAIR_REQUEST_EVENT = "remote://pair-request";

/** Every role, with its name and what it allows, for the operator to choose from. */
export const ROLES: { role: Role; name: MessageKey; description: MessageKey }[] = [
  { role: "full", name: "role.full", description: "role.fullHint" },
  { role: "presenter", name: "role.presenter", description: "role.presenterHint" },
  { role: "worshipLeader", name: "role.worshipLeader", description: "role.worshipLeaderHint" },
  { role: "alerts", name: "role.alerts", description: "role.alertsHint" },
  { role: "stage", name: "role.stage", description: "role.stageHint" },
];

class RemoteStore {
  status: RemoteStatus | null = $state.raw(null);
  /** Phones waiting for the operator to allow them, oldest first. */
  requests: PairRequest[] = $state([]);
  /** Whether the "Connect a Phone" window is open. */
  connecting = $state(false);

  /** How many phones are connected now. */
  get connectedCount(): number {
    return this.status?.connected.length ?? 0;
  }

  /** Starts following the remote. Resolves to a function that stops. */
  async connect(): Promise<() => void> {
    const stopStatus = await listen<RemoteStatus>(STATUS_EVENT, ({ payload }) => {
      this.status = payload;
    });
    const stopRequests = await listen<PairRequest>(PAIR_REQUEST_EVENT, ({ payload }) => {
      this.requests = [...this.requests, payload];
    });
    this.status = await invoke<RemoteStatus>("remote_status");
    return () => {
      stopStatus();
      stopRequests();
    };
  }

  /** Turns the remote on or off. */
  async setEnabled(enabled: boolean, port: number): Promise<void> {
    try {
      this.status = await invoke<RemoteStatus>("remote_set_enabled", { enabled, port });
    } catch (error) {
      engine.showError(error);
    }
  }

  /** A new pairing code for "Connect a Phone". */
  startPairing(): Promise<PairingCode> {
    return invoke<PairingCode>("remote_pairing_start");
  }

  cancelPairing(): void {
    invoke("remote_pairing_cancel").catch(() => {});
  }

  /** Allows a phone as `role`, or refuses it (`null`). */
  async answer(request: PairRequest, role: Role | null): Promise<void> {
    this.requests = this.requests.filter((waiting) => waiting.id !== request.id);
    if (role) this.connecting = false;
    await invoke("remote_answer", { request: request.id, role }).catch((error) =>
      engine.showError(error),
    );
  }

  setRole(device: string, role: Role): void {
    invoke("remote_set_role", { device, role }).catch((error) => engine.showError(error));
  }

  remove(device: string): void {
    invoke("remote_remove", { device }).catch((error) => engine.showError(error));
  }
}

export const remote = new RemoteStore();
