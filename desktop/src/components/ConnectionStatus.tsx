import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";
import { useConnectionActivity } from "@/hooks/useConnectionActivity";
import { connectionIndicator } from "@/lib/connectionIndicator";

export function ConnectionStatus() {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const { configuration, activity, failed } = useConnectionActivity();
  const enabled = getSetting("post_process_enabled") ?? false;
  const selected = configuration?.connections.find(
    (c) => c.id === configuration.active_connection_id,
  );
  const status = activity.find((c) => c.connection_id === selected?.id);
  const indicator = connectionIndicator({
    configurationLoaded: configuration !== null,
    readFailed: failed,
    enabled,
    hasSelection: selected !== undefined,
    testSucceeded: status?.test_success ?? null,
  });
  return (
    <section
      className="mt-auto shrink-0 min-w-0 w-full border-t border-mid-gray/20 p-2 space-y-1 text-xs break-words"
      aria-label={t("notype.usability.connectionStatus")}
    >
      <h3 className="font-semibold">{t("notype.connectionIndicator.title")}</h3>
      <p className="flex gap-2 items-start">
        <span
          role="img"
          aria-label={t("notype.connectionIndicator.states." + indicator.state)}
          data-connection-state={indicator.state}
          className={`mt-1 w-2 h-2 rounded-full shrink-0 ${indicator.color}`}
        />
        <span className="font-medium">
          {selected?.display_name ?? t("notype.connectionIndicator.noServer")}
        </span>
      </p>
      <p>{t(enabled ? "notype.usability.aiOn" : "notype.usability.aiOff")}</p>
      {selected && (
        <p
          className="break-all text-text/60"
          title={selected.effective_endpoint ?? selected.api_address}
        >
          {selected.effective_endpoint ?? selected.api_address}
        </p>
      )}
      {selected && <p>{selected.model}</p>}
      <p role="status">
        {t(
          failed
            ? "notype.usability.activityUnavailable"
            : status?.test_success === true
              ? "notype.usability.testOk"
              : status?.test_success === false
                ? "notype.usability.testFailed"
                : "notype.usability.notTested",
        )}
      </p>
    </section>
  );
}
