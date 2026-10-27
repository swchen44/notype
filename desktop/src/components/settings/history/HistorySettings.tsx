import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type SessionDto, type WorkflowReadiness } from "@/bindings";
import {
  historyText,
  historyMatches,
  historySegments,
} from "@/lib/historyPresentation";
import { HistoryMetrics } from "./HistoryMetrics";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";

export const HistorySettings = () => {
  const { t, i18n } = useTranslation();
  const [exporting, setExporting] = useState(false);
  const [exportMessage, setExportMessage] = useState("");
  const exportLock = useRef(false);
  const exportAll = async () => {
    if (exportLock.current) return;
    exportLock.current = true;
    setExporting(true);
    setExportMessage("");
    try {
      const result = await commands.exportHistoryCsv();
      if (result.status !== "ok") throw new Error("history-export-failed");
      setExportMessage(
        result.data.status === "cancelled"
          ? t("notype.history.exportCancelled")
          : t("notype.history.exportSaved", { rows: result.data.rows }),
      );
    } catch {
      setExportMessage(t("notype.history.exportFailed"));
    } finally {
      exportLock.current = false;
      setExporting(false);
    }
  };
  const [query, setQuery] = useState("");
  const [entries, setEntries] = useState<SessionDto[]>([]);
  const [readiness, setReadiness] = useState<WorkflowReadiness | null>(null);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);
  const [working, setWorking] = useState(new Set<string>());
  const [copied, setCopied] = useState<{
    id: string;
    version: "original" | "result" | "back_translation";
  } | null>(null);
  const request = useRef(0);
  const refresh = useCallback(async () => {
    const generation = ++request.current;
    try {
      const [result, capabilities] = await Promise.all([
        commands.getHistoryEntries(query),
        commands.getWorkflowReadiness(),
      ]);
      if (generation !== request.current) return;
      if (result.status !== "ok") throw new Error("history-unavailable");
      setEntries(result.data);
      setReadiness(capabilities);
      setFailed(false);
    } catch {
      if (generation === request.current) setFailed(true);
    } finally {
      if (generation === request.current) setLoading(false);
    }
  }, [query]);
  useEffect(() => {
    setLoading(true);
    void refresh();
    const unlisten = listen("session-history-changed", () => {
      void refresh();
    });
    return () => {
      ++request.current;
      void unlisten.then((stop) => stop());
    };
  }, [refresh]);
  useEffect(() => {
    if (!entries.some((entry) => entry.status === "processing")) return;
    const timer = window.setInterval(() => void refresh(), 500);
    return () => window.clearInterval(timer);
  }, [entries, refresh]);

  const run = async (
    id: string,
    operation: () => Promise<{ status: "ok" | "error"; error?: string }>,
  ) => {
    setWorking((previous) => new Set(previous).add(id));
    try {
      const result = await operation();
      if (result.status !== "ok" && result.error !== "session-not-found")
        toast.error(t("notype.history.actionFailed"));
    } catch {
      toast.error(t("notype.history.actionFailed"));
    } finally {
      setWorking((previous) => {
        const next = new Set(previous);
        next.delete(id);
        return next;
      });
      await refresh();
    }
  };
  const copy = async (
    entry: SessionDto,
    version: "original" | "result" | "back_translation",
  ) => {
    try {
      const result = await commands.copyHistoryText(entry.id, version);
      if (result.status === "ok") setCopied({ id: entry.id, version });
      else toast.error(t("notype.history.copyFailed"));
    } catch {
      toast.error(t("notype.history.copyFailed"));
    }
  };

  const visibleEntries = entries.filter((entry) =>
    historyMatches(historyText(entry), query),
  );
  const errorDescription = (error: string) =>
    i18n.exists(`notype.errors.${error}`)
      ? t(`notype.errors.${error}`)
      : t("notype.history.failed");
  return (
    <section
      className="max-w-3xl w-full mx-auto space-y-4"
      aria-label={t("notype.history.title")}
    >
      <h2 className="font-semibold">{t("notype.history.title")}</h2>
      <p className="text-sm text-text/60">{t("notype.history.retained")}</p>
      <Button
        variant="secondary"
        disabled={exporting}
        onClick={() => void exportAll()}
      >
        {t(exporting ? "notype.history.exporting" : "notype.history.exportAll")}
      </Button>
      <p role="status" className="text-sm">
        {exportMessage}
      </p>
      <Input
        type="search"
        className="w-full"
        aria-label={t("notype.history.search")}
        placeholder={t("notype.history.search")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
      />
      {readiness && !readiness.delivery_available && (
        <p role="note" className="text-sm text-text/70">
          {t("notype.history.deliveryUnavailable")}
        </p>
      )}
      {readiness?.legacy_import_error && (
        <p role="alert">{t("notype.history.importFailed")}</p>
      )}
      {failed ? (
        <p role="alert">{t("notype.history.actionFailed")}</p>
      ) : loading ? (
        <p>{t("notype.history.loading")}</p>
      ) : visibleEntries.length === 0 ? (
        <p>{t(query ? "notype.history.noMatches" : "notype.history.empty")}</p>
      ) : (
        visibleEntries.map((entry) => {
          const active =
            entry.status === "processing" || entry.status === "delivering";
          const frozen =
            entry.status === "delivered" || entry.status === "delivery-unknown";
          const canChoose = [
            "recognized",
            "failed",
            "pending",
            "cancelled",
          ].includes(entry.status);
          return (
            <article
              key={entry.id}
              data-session-id={entry.id}
              className="border border-mid-gray/30 rounded-lg p-4 space-y-3"
            >
              <time
                className="block text-sm text-text/60"
                dateTime={entry.created_at}
              >
                {new Date(entry.created_at).toLocaleString(i18n.language)}
              </time>
              <p className="whitespace-pre-wrap break-words select-text cursor-text">
                {historySegments(historyText(entry), query).map(
                  (part, index) =>
                    part.match ? (
                      <mark
                        key={index}
                        className="bg-logo-primary/30 text-text rounded-sm"
                      >
                        {part.text}
                      </mark>
                    ) : (
                      <span key={index}>{part.text}</span>
                    ),
                )}
              </p>
              <details className="history-advanced space-y-3">
                <summary className="cursor-pointer text-sm text-text/70">
                  {t("notype.usability.advanced")}
                </summary>
                <div className="flex flex-wrap gap-2 justify-between text-sm text-text/70">
                  <span>
                    {t(`notype.scenarios.${entry.scenario}`)}
                    {entry.translation_target
                      ? ` → ${entry.translation_target}`
                      : ""}
                  </span>
                  <time dateTime={entry.created_at}>
                    {new Date(entry.created_at).toLocaleString(i18n.language)}
                  </time>
                  <span role="status">
                    {t(`notype.status.${entry.status}`)}
                  </span>
                </div>
                {entry.status === "delivery-unknown" && (
                  <p role="alert">{t("notype.history.uncertain")}</p>
                )}
                {entry.observations.latest_processing_error && (
                  <p
                    className="text-sm"
                    role="note"
                    aria-label={t("notype.usability.processingError")}
                  >
                    <strong>{t("notype.usability.processingError")}: </strong>
                    <span>
                      {errorDescription(
                        entry.observations.latest_processing_error,
                      )}
                    </span>
                  </p>
                )}
                {entry.error &&
                  entry.error !== entry.observations.latest_processing_error &&
                  entry.status !== "delivery-unknown" && (
                    <p className="text-sm" role="note">
                      {errorDescription(entry.error)}
                    </p>
                  )}
                <div>
                  <h3 className="text-sm font-semibold">
                    {t("notype.history.original")}
                  </h3>
                  <p className="text-sm">
                    {t("notype.metrics.originalCount", {
                      value:
                        entry.observations.original_count ??
                        t("notype.metrics.unknown"),
                    })}
                  </p>
                  <p className="whitespace-pre-wrap break-words select-text cursor-text text-sm mt-1">
                    {entry.transcript}
                  </p>
                  <Button
                    variant="secondary"
                    size="sm"
                    className="mt-2"
                    onClick={() => {
                      void copy(entry, "original");
                    }}
                  >
                    {t("notype.history.copyOriginal")}
                  </Button>
                </div>
                <div>
                  <h3 className="text-sm font-semibold">
                    {t(
                      entry.scenario === "translation"
                        ? "notype.scenarios.primaryTranslation"
                        : "notype.history.result",
                    )}
                  </h3>
                  <p className="text-sm">
                    {t("notype.metrics.resultCount", {
                      value:
                        entry.observations.result_count ??
                        t("notype.metrics.unknown"),
                    })}
                  </p>
                  <p className="whitespace-pre-wrap break-words select-text cursor-text text-sm mt-1">
                    {entry.result ?? t("notype.history.noResult")}
                  </p>
                  <Button
                    variant="secondary"
                    size="sm"
                    className="mt-2"
                    disabled={entry.result === null}
                    onClick={() => {
                      void copy(entry, "result");
                    }}
                  >
                    {t("notype.history.copyResult")}
                  </Button>
                </div>
                <HistoryMetrics entry={entry} />
                {entry.review_required && (
                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold">
                      {t("notype.scenarios.backResult")}
                      {entry.source_language
                        ? ` → ${entry.source_language}`
                        : ""}
                    </h3>
                    <p className="whitespace-pre-wrap break-words select-text cursor-text text-sm">
                      {entry.back_translation ??
                        t(
                          entry.status === "processing"
                            ? "notype.scenarios.backPending"
                            : "notype.scenarios.backMissing",
                        )}
                    </p>
                    {entry.back_translation && (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => void copy(entry, "back_translation")}
                      >
                        {t("notype.scenarios.copyBack")}
                      </Button>
                    )}
                    <p className="text-sm" role="note">
                      {t(
                        entry.review_confirmed
                          ? "notype.scenarios.reviewConfirmed"
                          : "notype.scenarios.reviewRequired",
                      )}
                    </p>
                    {entry.status === "pending" &&
                      !entry.review_confirmed &&
                      (entry.back_translation ||
                        entry.selection === "original") && (
                        <Button
                          variant="secondary"
                          size="sm"
                          disabled={working.has(entry.id)}
                          onClick={() =>
                            void run(entry.id, () =>
                              commands.confirmHistoryReview(
                                entry.id,
                                entry.revision,
                              ),
                            )
                          }
                        >
                          {t("notype.scenarios.confirmReview")}
                        </Button>
                      )}
                  </div>
                )}
                {copied?.id === entry.id && (
                  <p role="status" className="text-sm">
                    {t(
                      copied.version === "original"
                        ? "notype.history.copiedOriginal"
                        : copied.version === "back_translation"
                          ? "notype.scenarios.copiedBack"
                          : "notype.history.copiedResult",
                    )}
                  </p>
                )}
                {entry.selection === "original" && (
                  <p className="text-sm">
                    {t("notype.history.selectedOriginal")}
                  </p>
                )}
                <div className="flex flex-wrap gap-2">
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={active || frozen || working.has(entry.id)}
                    onClick={() => {
                      void run(entry.id, () =>
                        commands.retryHistoryEntryProcessing(entry.id),
                      );
                    }}
                  >
                    {t("notype.history.retry")}
                  </Button>
                  {(active || working.has(entry.id)) && (
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={() => {
                        void run(entry.id, () =>
                          commands.cancelHistoryEntry(entry.id),
                        );
                      }}
                    >
                      {t("notype.history.cancel")}
                    </Button>
                  )}
                  {canChoose && entry.selection !== "original" && (
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={working.has(entry.id)}
                      onClick={() => {
                        void run(entry.id, () =>
                          commands.chooseHistoryOriginal(entry.id),
                        );
                      }}
                    >
                      {t("notype.history.chooseOriginal")}
                    </Button>
                  )}
                  {entry.status === "pending" && (
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={
                        !readiness?.delivery_available ||
                        working.has(entry.id) ||
                        (entry.review_required && !entry.review_confirmed)
                      }
                      onClick={() => {
                        void run(entry.id, () =>
                          commands.deliverHistoryEntry(entry.id),
                        );
                      }}
                    >
                      {t("notype.history.deliver")}
                    </Button>
                  )}
                  <Button
                    variant="secondary"
                    size="sm"
                    disabled={entry.status === "delivering"}
                    onClick={() => {
                      void run(entry.id, () =>
                        commands.deleteHistoryEntry(entry.id),
                      );
                    }}
                  >
                    {t("notype.history.delete")}
                  </Button>
                </div>
              </details>
            </article>
          );
        })
      )}
    </section>
  );
};
