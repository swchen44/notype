import { useTranslation } from "react-i18next";
import type { HistoryRequestParametersDto } from "@/bindings";

/** Keep user preferences, actual request fields and documentation separate. */
export function HistoryRequestOptions({
  parameters,
}: {
  parameters: HistoryRequestParametersDto | null;
}) {
  const { t, i18n } = useTranslation();
  if (!parameters) return <p>{t("notype.requestOptions.unavailable")}</p>;
  const omitted = t("notype.requestOptions.omitted");
  const state = (value: boolean | null) =>
    value === null
      ? t("notype.metrics.unknown")
      : t("notype.requestOptions." + (value ? "on" : "off"));
  const category = (group: string, value: string) => {
    const key = "notype.requestOptions." + group + "." + value;
    return i18n.exists(key) ? t(key) : t("notype.metrics.unknown");
  };
  return (
    <div className="space-y-1 border-l border-mid-gray/30 pl-3">
      <p className="text-text/70">{t("notype.requestOptions.explanation")}</p>
      <p>
        {t("notype.requestOptions.thinkingPreference", {
          value: state(parameters.requested_thinking),
        })}
      </p>
      <p>
        {t("notype.requestOptions.thinkingWire", {
          value: state(parameters.effective_thinking),
        })}
      </p>
      <p>
        {t("notype.requestOptions.effortPreference", {
          value: parameters.preference_effort ?? omitted,
        })}
      </p>
      <p>
        {t("notype.requestOptions.effortWire", {
          value: parameters.effective_effort ?? omitted,
        })}
      </p>
      <p>
        {t("notype.requestOptions.provenance", {
          value: category("sources", parameters.capability_provenance),
        })}
      </p>
      <p>
        {t("notype.requestOptions.reason", {
          value: category("reasons", parameters.resolution_reason),
        })}
      </p>
      <p>
        {t("notype.requestOptions.temperaturePreference", {
          value: parameters.configured_temperature ?? omitted,
        })}
      </p>
      <p>
        {t("notype.requestOptions.temperatureWire", {
          value: parameters.wire_temperature ?? omitted,
        })}
      </p>
      <p>
        {t("notype.requestOptions.temperatureResolution", {
          value: category("sampling", parameters.temperature_resolution),
        })}
      </p>
      <p>
        {t("notype.requestOptions.topPPreference", {
          value: parameters.configured_top_p ?? omitted,
        })}
      </p>
      <p>
        {t("notype.requestOptions.topPWire", {
          value: parameters.wire_top_p ?? omitted,
        })}
      </p>
      <p>
        {t("notype.requestOptions.topPResolution", {
          value: category("sampling", parameters.top_p_resolution),
        })}
      </p>
      {parameters.documented_top_p !== null && (
        <p>
          {t("notype.requestOptions.documentedTopP", {
            value: parameters.documented_top_p,
          })}
        </p>
      )}
      <p>
        {t("notype.requestOptions.outputWire", {
          value: parameters.max_output_tokens ?? omitted,
        })}
      </p>
    </div>
  );
}
