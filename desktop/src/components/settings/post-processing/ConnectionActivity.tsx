import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type ConnectionActivityView,
  type HistoryUsageDto,
} from "@/bindings";
import { connectionConfigurationChanged } from "@/hooks/useConnectionActivity";
import { Button } from "../../ui/Button";

// Explicit connection tests have no speech session or saved transcript.
const TEST_ERROR_KEYS = new Map<string, string>([
  ["provider-auth", "auth"],
  ["provider-permission", "permission"],
  ["provider-rate-limit", "rateLimit"],
  ["provider-balance", "balance"],
  ["provider-context", "context"],
  ["provider-timeout", "timeout"],
  ["processing-timeout", "timeout"],
  ["provider-validation", "validation"],
  ["provider-unavailable", "unavailable"],
  ["provider-transport", "transport"],
  ["provider-protocol", "protocol"],
  ["processing-invalid-response", "protocol"],
  ["provider-oversize", "oversize"],
  ["processing-cancelled", "cancelled"],
  ["configuration-required", "configuration"],
  ["configuration-credentials-unavailable", "credentials"],
  ["configuration-capability-required", "options"],
  ["configuration-route-invalid", "validation"],
  ["configuration-invalid-options", "options"],
  ["configuration-storage-failed", "storage"],
  ["history-storage-failed", "storage"],
]);

export function ConnectionActivity({
  id,
  revision,
  activity,
  refresh,
}: {
  id: string;
  revision: number;
  activity?: ConnectionActivityView;
  refresh: () => Promise<void>;
}) {
  activity =
    activity?.configuration_revision === revision ? activity : undefined;
  const { t } = useTranslation();
  const [operation, setOperation] = useState<"test" | "clear" | null>(null),
    [message, setMessage] = useState("");
  const busy = operation !== null;
  const current = useRef({ id, revision });
  current.current = { id, revision };
  useEffect(() => setMessage(""), [id, revision]);
  const lock = useRef(false);
  const run = async (test: boolean) => {
    if (lock.current) return;
    lock.current = true;
    setOperation(test ? "test" : "clear");
    setMessage("");
    try {
      const result = test
        ? await commands.testTextConnection(revision, id)
        : await commands.clearConnectionUsage(id);
      if (current.current.id === id && current.current.revision === revision)
        setMessage(
          t(
            result.status === "ok"
              ? test
                ? "notype.usability.testOk"
                : "notype.usability.usageCleared"
              : test
                ? "notype.usability.testFailed"
                : "notype.usability.usageClearFailed",
          ),
        );
    } catch {
      if (current.current.id === id && current.current.revision === revision)
        setMessage(
          t(
            test
              ? "notype.usability.testFailed"
              : "notype.usability.usageClearFailed",
          ),
        );
    } finally {
      lock.current = false;
      setOperation(null);
      await refresh();
      connectionConfigurationChanged();
    }
  };
  const count = (value?: HistoryUsageDto) =>
    value?.status === "complete"
      ? String(value.total ?? 0)
      : value?.known_subtotal !== null && value?.known_subtotal !== undefined
        ? `${value.known_subtotal} + ${t("notype.metrics.unknown")}`
        : t("notype.metrics.unknown");
  const fees = activity?.summary.fees;
  const noCalls =
    activity?.summary.call_count === 0 &&
    activity.summary.observed_call_count === 0;
  return (
    <div className="space-y-2 text-sm">
      <p className="text-xs text-text/60">{t("notype.usability.usageScope")}</p>
      <p>
        {t("notype.usability.tokenInput")}:{" "}
        {noCalls ? "0" : count(activity?.summary.input)} ·{" "}
        {t("notype.usability.tokenOutput")}:{" "}
        {noCalls ? "0" : count(activity?.summary.output)}
      </p>
      <p>
        {t("notype.usability.actualFees")}:{" "}
        {noCalls || (fees?.status === "complete" && !fees.totals.length)
          ? "0"
          : (fees?.status === "complete" ? fees.totals : fees?.known_subtotals)
              ?.map((v) => `${v.currency} ${v.amount}`)
              .join(" + ") || t("notype.metrics.unknown")}
        {fees?.status === "partial" ? ` + ${t("notype.metrics.unknown")}` : ""}
      </p>
      <p>
        {t(
          activity?.test_success === true
            ? "notype.usability.testOk"
            : activity?.test_success === false
              ? "notype.usability.testFailed"
              : "notype.usability.notTested",
        )}
        {activity?.tested_at_ms
          ? ` · ${new Date(activity.tested_at_ms).toLocaleString()}`
          : ""}
      </p>
      {activity?.test_error && (
        <p role="note">
          {t(
            `notype.usability.testErrors.${TEST_ERROR_KEYS.get(activity.test_error) ?? "unknown"}`,
          )}
        </p>
      )}
      <div className="flex gap-2">
        <Button size="sm" disabled={busy} onClick={() => void run(true)}>
          {t(
            operation === "test"
              ? "notype.usability.testing"
              : "notype.usability.testConnection",
          )}
        </Button>
        <Button
          size="sm"
          variant="secondary"
          disabled={busy}
          onClick={() => void run(false)}
        >
          {t(
            operation === "clear"
              ? "notype.usability.clearingUsage"
              : "notype.usability.clearUsage",
          )}
        </Button>
      </div>
      <p className="text-xs text-text/60">{t("notype.usability.testCost")}</p>
      <p role="status">{message}</p>
    </div>
  );
}
