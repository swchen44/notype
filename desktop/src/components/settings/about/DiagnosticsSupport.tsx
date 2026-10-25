import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Button } from "../../ui/Button";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { LogLevelSelector } from "../debug/LogLevelSelector";
import {
  createDiagnosticIssueUrl,
  type DiagnosticPreview,
} from "../../../lib/diagnosticIssue";

interface CollectionStatus {
  initialized: boolean;
  effective_enabled: boolean;
  saved_preference: boolean | null;
  desired_enabled: boolean | null;
  persistence: string;
  revision: number;
  epoch: number;
}
type Preview = DiagnosticPreview & { epoch: number };
interface ExportOutcome {
  status: "saved" | "cancelled";
  report_id: string;
  epoch: number;
}

export const DiagnosticsSupport: React.FC = () => {
  const { t } = useTranslation();
  const [report, setReport] = useState<Preview | null>(null);
  const [busy, setBusy] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [collection, setCollection] = useState<CollectionStatus | null>(null);
  const [statusLoading, setStatusLoading] = useState(true);
  const [statusFailed, setStatusFailed] = useState(false);
  const [toggleBusy, setToggleBusy] = useState(false);
  const viewGeneration = useRef(0);
  const controlRequest = useRef(0);
  const preferenceRevision = useRef(-1);
  const epoch = useRef(0);
  const [message, setMessage] = useState("");

  function acknowledge(status: CollectionStatus) {
    if (status.revision < preferenceRevision.current) return;
    preferenceRevision.current = status.revision;
    epoch.current = Math.max(epoch.current, status.epoch);
    setCollection(status);
  }
  async function refreshStatus() {
    const request = ++controlRequest.current;
    setStatusLoading(true);
    setStatusFailed(false);
    try {
      const status = await invoke<CollectionStatus>("get_diagnostic_status");
      if (request === controlRequest.current) acknowledge(status);
    } catch {
      if (request === controlRequest.current) setStatusFailed(true);
    } finally {
      if (request === controlRequest.current) setStatusLoading(false);
    }
  }
  useEffect(() => {
    void refreshStatus();
    return () => {
      controlRequest.current++;
      viewGeneration.current++;
    };
  }, []);
  async function toggleCollection(enabled: boolean) {
    const request = ++controlRequest.current;
    setToggleBusy(true);
    setStatusFailed(false);
    try {
      const status = await invoke<CollectionStatus>("set_diagnostics_enabled", {
        enabled,
      });
      if (request === controlRequest.current) acknowledge(status);
    } catch {
      if (request === controlRequest.current) setStatusFailed(true);
    } finally {
      if (request === controlRequest.current) {
        setToggleBusy(false);
        setStatusLoading(false);
      }
    }
  }
  async function clear() {
    const generation = ++viewGeneration.current;
    setReport(null);
    setBusy(false);
    setClearing(true);
    setMessage(t("notype.diagnostics.clearing"));
    try {
      const clearedEpoch = await invoke<number>("clear_diagnostics");
      if (generation === viewGeneration.current) {
        epoch.current = Math.max(epoch.current, clearedEpoch);
        setMessage(t("notype.diagnostics.cleared"));
      }
    } catch {
      if (generation === viewGeneration.current)
        setMessage(t("notype.diagnostics.clearFailed"));
    } finally {
      if (generation === viewGeneration.current) setClearing(false);
    }
  }

  async function preview() {
    const generation = viewGeneration.current;
    setBusy(true);
    setMessage("");
    try {
      const response = await invoke<Preview>("get_diagnostic_report");
      if (
        generation === viewGeneration.current &&
        response.epoch >= epoch.current
      ) {
        epoch.current = response.epoch;
        setReport(response);
      }
    } catch {
      if (generation === viewGeneration.current)
        setMessage(t("notype.diagnostics.previewFailed"));
    } finally {
      if (generation === viewGeneration.current) setBusy(false);
    }
  }

  async function exportReport() {
    if (!report) return;
    const generation = viewGeneration.current;
    setBusy(true);
    setMessage("");
    try {
      const result = await invoke<ExportOutcome>("export_diagnostic_report", {
        reportId: report.report_id,
      });
      if (
        generation === viewGeneration.current &&
        result.epoch >= epoch.current &&
        result.report_id === report.report_id
      )
        setMessage(
          t(
            result.status === "saved"
              ? "notype.diagnostics.saved"
              : "notype.diagnostics.cancelled",
          ),
        );
    } catch (error) {
      if (generation === viewGeneration.current)
        setMessage(
          t(
            error === "diagnostic-preview-expired"
              ? "notype.diagnostics.expired"
              : error === "diagnostic-extension-invalid"
                ? "notype.diagnostics.extensionInvalid"
                : "notype.diagnostics.exportFailed",
          ),
        );
    } finally {
      if (generation === viewGeneration.current) setBusy(false);
    }
  }

  async function openIssue() {
    if (!report) return;
    const generation = viewGeneration.current;
    setBusy(true);
    setMessage("");
    try {
      await openUrl(createDiagnosticIssueUrl(report));
    } catch {
      if (generation === viewGeneration.current)
        setMessage(t("notype.diagnostics.issueFailed"));
    } finally {
      if (generation === viewGeneration.current) setBusy(false);
    }
  }

  return (
    <SettingsGroup title={t("notype.diagnostics.title")}>
      <div className="space-y-3 px-4 py-3 text-sm">
        <p>{t("notype.diagnostics.scope")}</p>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={collection?.effective_enabled ?? false}
            disabled={
              !collection?.initialized ||
              toggleBusy ||
              collection.persistence === "pending"
            }
            onChange={(event) => void toggleCollection(event.target.checked)}
          />
          {t("notype.diagnostics.collect")}
        </label>
        <p role="status" aria-live="polite">
          {t(
            statusFailed
              ? "notype.diagnostics.statusFailed"
              : toggleBusy || collection?.persistence === "pending"
                ? "notype.diagnostics.savingPreference"
                : !collection?.initialized
                  ? "notype.diagnostics.loadingStatus"
                  : collection.persistence === "unsaved"
                    ? "notype.diagnostics.unsavedOff"
                    : collection.persistence === "load_failed"
                      ? "notype.diagnostics.loadFailed"
                      : collection.effective_enabled
                        ? "notype.diagnostics.collectionOn"
                        : "notype.diagnostics.collectionOff",
          )}
        </p>
        <Button
          variant="secondary"
          onClick={() => void refreshStatus()}
          disabled={statusLoading || toggleBusy}
        >
          {t("notype.diagnostics.refreshStatus")}
        </Button>
        {(collection?.persistence === "unsaved" ||
          collection?.persistence === "load_failed") && (
          <Button
            variant="secondary"
            disabled={toggleBusy}
            onClick={() =>
              void toggleCollection(
                collection.desired_enabled ?? collection.effective_enabled,
              )
            }
          >
            {t("notype.diagnostics.retryPreference")}
          </Button>
        )}
        <p>{t("notype.diagnostics.description")}</p>
        <ol className="list-decimal pl-5 space-y-2">
          <li>{t("notype.diagnostics.reproduce")}</li>
          <li>{t("notype.diagnostics.review")}</li>
          <li>{t("notype.diagnostics.attach")}</li>
        </ol>
        <p className="text-mid-gray">{t("notype.diagnostics.privacy")}</p>
        <div className="flex flex-wrap gap-2">
          <Button
            onClick={() => void preview()}
            disabled={busy || clearing || !collection?.initialized}
          >
            {t("notype.diagnostics.preview")}
          </Button>
          <Button
            variant="secondary"
            onClick={() => void exportReport()}
            disabled={busy || clearing || !report}
          >
            {t("notype.diagnostics.export")}
          </Button>
          <Button
            variant="secondary"
            onClick={() => void openIssue()}
            disabled={busy || clearing || !report}
          >
            {t("notype.diagnostics.openIssue")}
          </Button>
          <Button
            variant="secondary"
            onClick={() => void clear()}
            disabled={clearing || !collection?.initialized}
          >
            {t("notype.diagnostics.clear")}
          </Button>
        </div>
        {report && (
          <label className="block space-y-1">
            <span>{t("notype.diagnostics.previewLabel")}</span>
            <textarea
              readOnly
              value={report.text}
              rows={12}
              className="w-full resize-y rounded-lg border border-mid-gray/20 bg-background p-3 font-mono text-xs select-text"
            />
          </label>
        )}
        <p role="status" aria-live="polite">
          {message}
        </p>
      </div>
      <LogLevelSelector grouped descriptionMode="inline" />
    </SettingsGroup>
  );
};
