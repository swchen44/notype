export type ConnectionIndicatorState =
  | "loading"
  | "unavailable"
  | "unselected"
  | "disabled"
  | "untested"
  | "succeeded"
  | "failed";

/** Describe current activation separately from the last observed test result. */
export function connectionIndicator(input: {
  configurationLoaded: boolean;
  readFailed: boolean;
  enabled: boolean;
  hasSelection: boolean;
  testSucceeded: boolean | null;
}): { state: ConnectionIndicatorState; color: string } {
  if (input.readFailed) return { state: "unavailable", color: "bg-amber-400" };
  if (!input.configurationLoaded)
    return { state: "loading", color: "bg-mid-gray/60" };
  if (!input.hasSelection)
    return { state: "unselected", color: "bg-mid-gray/60" };
  if (!input.enabled) return { state: "disabled", color: "bg-mid-gray/60" };
  if (input.testSucceeded === true)
    return { state: "succeeded", color: "bg-green-400" };
  if (input.testSucceeded === false)
    return { state: "failed", color: "bg-red-400" };
  return { state: "untested", color: "bg-amber-400" };
}
