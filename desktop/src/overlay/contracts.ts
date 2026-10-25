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
