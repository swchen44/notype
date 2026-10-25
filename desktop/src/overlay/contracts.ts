import { invoke } from "@tauri-apps/api/core";
/** Process-local workflow capture identity and overlay request revision.
 * Both are decimal u64 strings, never persistent history/target/device IDs.
 */
export type OverlayState =
  | "recording"
  | "streaming"
  | "transcribing"
  | "processing";
export type OverlayIdentity = { captureId: string; revision: string };
export type ShowOverlayEvent = OverlayIdentity & { state: OverlayState };
export type RecordingReadyEvent = {
  captureId: string;
  captureRevision: string;
};
export type HideOverlayEvent = OverlayIdentity & {
  reason: "closed" | "finished" | "cancelled" | "failed";
};

/** Only local elapsed differences cross this boundary; never performance.now(). */
export function reportCaptureObservation(
  captureId: string,
  captureRevision: string,
  observation: "received" | "committed" | "settings",
  durationMs: number,
): Promise<boolean> {
  if (!Number.isFinite(durationMs) || durationMs > Number.MAX_SAFE_INTEGER)
    return Promise.resolve(false);
  return invoke<boolean>("report_capture_observation", {
    captureId,
    captureRevision,
    observation,
    durationMs: Math.max(0, Math.floor(durationMs)),
  });
}
