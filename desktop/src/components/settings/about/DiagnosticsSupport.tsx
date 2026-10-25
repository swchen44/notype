import React, { useState } from "react";
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

export const DiagnosticsSupport: React.FC = () => {
  const { t } = useTranslation();
  const [report, setReport] = useState<DiagnosticPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  async function preview() {
    setBusy(true);
    setMessage("");
    try {
      setReport(await invoke<DiagnosticPreview>("get_diagnostic_report"));
    } catch {
      setMessage(t("notype.diagnostics.previewFailed"));
    } finally {
      setBusy(false);
    }
  }

  async function exportReport() {
    if (!report) return;
    setBusy(true);
    setMessage("");
    try {
      const saved = await invoke<boolean>("export_diagnostic_report", {
        reportId: report.report_id,
      });
      setMessage(
        t(saved ? "notype.diagnostics.saved" : "notype.diagnostics.cancelled"),
      );
    } catch (error) {
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
      setBusy(false);
    }
  }

  async function openIssue() {
    if (!report) return;
    setMessage("");
    try {
      await openUrl(createDiagnosticIssueUrl(report));
    } catch {
      setMessage(t("notype.diagnostics.issueFailed"));
    }
  }

  return (
    <SettingsGroup title={t("notype.diagnostics.title")}>
      <div className="space-y-3 px-4 py-3 text-sm">
        <p>{t("notype.diagnostics.description")}</p>
        <ol className="list-decimal pl-5 space-y-2">
          <li>{t("notype.diagnostics.reproduce")}</li>
          <li>{t("notype.diagnostics.review")}</li>
          <li>{t("notype.diagnostics.attach")}</li>
        </ol>
        <p className="text-mid-gray">{t("notype.diagnostics.privacy")}</p>
        <div className="flex flex-wrap gap-2">
          <Button onClick={() => void preview()} disabled={busy}>
            {t("notype.diagnostics.preview")}
          </Button>
          <Button
            variant="secondary"
            onClick={() => void exportReport()}
            disabled={busy || !report}
          >
            {t("notype.diagnostics.export")}
          </Button>
          <Button
            variant="secondary"
            onClick={() => void openIssue()}
            disabled={busy || !report}
          >
            {t("notype.diagnostics.openIssue")}
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
