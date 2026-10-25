import { useTranslation } from "react-i18next";
import type { SessionDto, HistoryUsageDto } from "@/bindings";

/** Present the authoritative history projection without recounting or totaling. */
export function HistoryMetrics({ entry }: { entry: SessionDto }) {
  const { t } = useTranslation();
  const m = entry.observations;
  const api = entry.api_summary;
  const unknown = t("notype.metrics.unknown");
  const value = (value: number | string | null) => value ?? unknown;
  const status = (value: string) =>
    t(`notype.metrics.states.${value}`, { defaultValue: unknown });
  const ms = (value: number | null) =>
    value === null ? unknown : `${value} ms`;
  const usage = (metric: HistoryUsageDto) => (
    <>
      <p>
        {metric.status === "complete"
          ? t("notype.metrics.completeTotal", { value: value(metric.total) })
          : metric.status === "partial"
            ? t("notype.metrics.partialSubtotal", {
                value: value(metric.known_subtotal),
              })
            : status(metric.status)}
      </p>
      <p>
        {t("notype.metrics.reportedCalls", { value: metric.reported_calls })}
      </p>
    </>
  );
  return (
    <div className="text-sm space-y-3 min-w-0 break-words">
      <p>{t("notype.metrics.countRule", { value: m.count_rule })}</p>
      <p className="text-text/60">{t("notype.metrics.countExplanation")}</p>
      <dl className="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-1 [&_dd]:min-w-0 [&_dd]:break-words">
        <dt>{t("notype.metrics.audio")}</dt>
        <dd>
          {m.recording
            ? `${(m.recording.audio_ms / 1000).toFixed(3)} s · ${status(m.recording.quality)}`
            : unknown}
        </dd>
        <dt>{t("notype.metrics.requestedAsr")}</dt>
        <dd>{value(m.requested_asr_model)}</dd>
        <dt>{t("notype.metrics.actualAsr")}</dt>
        <dd>
          {m.actual_asr_model
            ? `${m.actual_asr_model.name ?? m.actual_asr_model.id} (${m.actual_asr_model.id}; ${m.actual_asr_model.backend ?? unknown})`
            : unknown}
        </dd>
        <dt>{t("notype.metrics.initialTime")}</dt>
        <dd>{ms(m.initial_stop_to_result_ms)}</dd>
        <dt>{t("notype.metrics.asrTime")}</dt>
        <dd>{ms(m.asr_after_stop_ms)}</dd>
        <dt>{t("notype.metrics.latestKind")}</dt>
        <dd>
          {m.latest_processing_kind === null
            ? unknown
            : status(m.latest_processing_kind)}
        </dd>
        <dt>{t("notype.metrics.latestTime")}</dt>
        <dd>{ms(m.latest_processing_ms)}</dd>
        <dt>{t("notype.metrics.apiTime")}</dt>
        <dd>{ms(m.latest_api_ms)}</dd>
        <dt>{t("notype.metrics.outcome")}</dt>
        <dd>
          {m.latest_outcome === null ? unknown : status(m.latest_outcome)}
        </dd>
      </dl>
      <details>
        <summary className="cursor-pointer">
          {t("notype.metrics.apiDetails")}
        </summary>
        <div className="space-y-3 mt-2">
          <p>
            {t("notype.metrics.callCount", { value: value(api.call_count) })}
          </p>
          <p>
            {t("notype.metrics.observedCalls", {
              value: api.observed_call_count,
            })}
          </p>
          {m.api_metadata_unavailable && (
            <p role="note">{t("notype.metrics.metadataUnavailable")}</p>
          )}
          <div>
            <h4 className="font-semibold">{t("notype.metrics.input")}</h4>
            {usage(api.input)}
          </div>
          <div>
            <h4 className="font-semibold">{t("notype.metrics.output")}</h4>
            {usage(api.output)}
          </div>
          <div>
            <h4 className="font-semibold">{t("notype.metrics.fees")}</h4>
            <p>
              {status(api.fees.status)}
              {api.fees.status === "partial"
                ? ` · ${t("notype.metrics.knownSubtotal")}`
                : ""}
            </p>
            {(api.fees.status === "complete"
              ? api.fees.totals
              : api.fees.known_subtotals
            ).map((money) => (
              <p key={money.currency}>
                {money.currency} {money.amount}
              </p>
            ))}
            <p>
              {t("notype.metrics.reportedCalls", {
                value: api.fees.reported_calls,
              })}
            </p>
          </div>
          <p>
            {t("notype.metrics.requestedModels")}:{" "}
            {api.requested_models.join(", ") || unknown}
          </p>
          <p>
            {t("notype.metrics.reportedModels")}:{" "}
            {api.reported_models.join(", ") || unknown}
          </p>
          <h4 className="font-semibold">{t("notype.metrics.associations")}</h4>
          {api.model_efforts.length === 0 ? (
            <p>{unknown}</p>
          ) : (
            <ul
              aria-label={t("notype.metrics.associations")}
              className="space-y-2"
            >
              {api.model_efforts.map((association, index) => (
                <li key={index}>
                  <p>
                    {t("notype.metrics.requestedPair", {
                      model: association.requested_model ?? unknown,
                      effort:
                        association.requested_effort ??
                        t("notype.metrics.providerDefault"),
                    })}
                  </p>
                  <p>
                    {t("notype.metrics.reportedPair", {
                      model: association.reported_model ?? unknown,
                      effort: association.reported_effort ?? unknown,
                    })}
                  </p>
                </li>
              ))}
            </ul>
          )}
        </div>
      </details>
    </div>
  );
}
