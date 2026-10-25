import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { invoke } from "@tauri-apps/api/core";
import { Check, Download, Loader2, Trash2 } from "lucide-react";
import type { ModelInfo } from "@/bindings";
import { formatModelSize } from "../../lib/utils/format";
import {
  getTranslatedModelDescription,
  getTranslatedModelName,
} from "../../lib/utils/modelTranslation";
import {
  getLanguageLabel,
  getUniqueCapabilityLanguages,
  supportsLanguageCode,
} from "../../lib/constants/languages";
import Badge from "../ui/Badge";
import { Button } from "../ui/Button";
import { useSettingsStore } from "@/stores/settingsStore";

export const isLegacySource = (model: ModelInfo): boolean =>
  typeof model.source === "object" && "Url" in model.source;
export type ModelCardStatus =
  | "downloadable"
  | "downloading"
  | "verifying"
  | "extracting"
  | "switching"
  | "active"
  | "available";
interface ModelCardProps {
  model: ModelInfo;
  variant?: "default" | "featured";
  status?: ModelCardStatus;
  disabled?: boolean;
  className?: string;
  onSelect: (modelId: string) => void;
  onDownload?: (modelId: string) => void;
  onDelete?: (modelId: string) => void;
  onCancel?: (modelId: string) => void;
  downloadProgress?: number;
  downloadSpeed?: number;
  downloadedBytes?: number;
  totalBytes?: number;
  showRecommended?: boolean;
}

const ModelCard: React.FC<ModelCardProps> = ({
  model,
  variant = "default",
  status = "downloadable",
  disabled = false,
  className = "",
  onSelect,
  onDownload,
  onDelete,
  onCancel,
  downloadProgress,
  downloadSpeed,
  downloadedBytes,
  totalBytes,
  showRecommended = true,
}) => {
  const { t, i18n } = useTranslation();
  const mode = useSettingsStore(
    (state) => state.settings?.recognition_mode ?? "traditional_english",
  );
  const [expanded, setExpanded] = useState(false);
  const [receiptError, setReceiptError] = useState(false);
  async function openQualityReceipt() {
    setReceiptError(false);
    try {
      await openUrl(
        "https://github.com/swchen44/notype/blob/main/docs/verification/qwen3-quality.md",
      );
    } catch {
      setReceiptError(true);
    }
  }
  const [modelPath, setModelPath] = useState<string | null>(null);
  const [pathState, setPathState] = useState<
    "idle" | "loading" | "ready" | "error"
  >("idle");
  const displayName = getTranslatedModelName(model, i18n);
  const displayDescription = getTranslatedModelDescription(model, i18n);
  const languages = getUniqueCapabilityLanguages(model.supported_languages);
  const compatible =
    supportsLanguageCode(model.supported_languages, "zh") &&
    supportsLanguageCode(model.supported_languages, "en");
  const quant = model.filename
    .match(/[._-](IQ\d+_\w+|Q\d+(?:_\w+)?|F16|BF16|F32)\.gguf$/i)?.[1]
    ?.toUpperCase();
  const source =
    typeof model.source === "object"
      ? "HuggingFace" in model.source
        ? `${model.source.HuggingFace.repo_id} @ ${model.source.HuggingFace.revision}`
        : model.source.Url.url
      : model.source;
  useEffect(() => {
    let cancelled = false;
    setModelPath(null);
    if (!expanded || !model.is_downloaded || model.is_downloading) {
      setPathState("idle");
      return;
    }
    setPathState("loading");
    invoke<string>("get_model_path", { modelId: model.id })
      .then((resolved) => {
        if (!cancelled) {
          setModelPath(resolved || null);
          setPathState(resolved ? "ready" : "error");
        }
      })
      .catch(() => {
        if (!cancelled) setPathState("error");
      });
    return () => {
      cancelled = true;
    };
  }, [expanded, model.id, model.is_downloaded, model.is_downloading]);
  const formatBytes = (bytes: number) =>
    `${(bytes / (1024 * 1024)).toLocaleString(i18n.language, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} MiB`;
  return (
    <article
      aria-label={displayName}
      className={`model-card ${status === "active" || variant === "featured" ? "model-card-emphasized" : ""} ${className}`}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-lg font-semibold break-words">{displayName}</h3>
            {showRecommended && model.is_recommended && (
              <Badge variant="primary">{t("onboarding.recommended")}</Badge>
            )}
            {status === "active" && (
              <Badge variant="primary">
                <Check className="w-3 h-3 inline mr-1" />
                {t("modelSelector.active")}
              </Badge>
            )}
          </div>
          <p className="text-sm text-text/75 mt-1 leading-relaxed">
            {displayDescription}
          </p>
        </div>
        <span className="text-sm tabular-nums whitespace-nowrap">
          {model.size_mb > 0
            ? formatModelSize(model.size_mb)
            : t("notype.modelExplorer.unknown")}
        </span>
      </div>
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-text/70">
        <span>
          {t("notype.modelExplorer.status", {
            status: t(`notype.modelExplorer.states.${status}`),
          })}
        </span>
        <span>
          {t("modelSelector.capabilities.languageCount", {
            total: languages.length,
          })}
        </span>
        {model.supports_streaming && (
          <span>{t("modelSelector.streaming")}</span>
        )}
        {model.is_custom && <span>{t("modelSelector.custom")}</span>}
        {isLegacySource(model) && <span>{t("modelSelector.legacy")}</span>}
      </div>
      <p className={`text-xs ${compatible ? "text-text/70" : "text-warning"}`}>
        {t(
          compatible
            ? "notype.modelExplorer.compatible"
            : "notype.modelExplorer.incompatible",
          {
            mode: t(
              mode === "traditional_english"
                ? "notype.recognition.traditional"
                : "notype.recognition.simplified",
            ),
          },
        )}
      </p>
      <div className="text-xs text-text/75">
        <p className="mb-1">{t("notype.modelExplorer.scores")}</p>
        <dl className="flex flex-wrap gap-x-6 gap-y-1 tabular-nums">
          {(
            [
              ["accuracy", model.accuracy_score],
              ["speed", model.speed_score],
            ] as const
          ).map(([kind, score]) => (
            <div key={kind} className="flex gap-2">
              <dt>{t(`onboarding.modelCard.${kind}`)}</dt>
              <dd className="font-semibold">
                {Number.isFinite(score) && score > 0 && score <= 1
                  ? score.toFixed(2)
                  : t("notype.modelExplorer.missingScore")}
              </dd>
            </div>
          ))}
        </dl>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {status === "downloadable" && onDownload && (
          <Button
            size="sm"
            disabled={disabled}
            onClick={() => onDownload(model.id)}
          >
            <Download className="w-3.5 h-3.5 inline mr-1" />
            {t("notype.modelExplorer.download")}
          </Button>
        )}
        {status === "available" && (
          <Button
            size="sm"
            disabled={disabled}
            onClick={() => onSelect(model.id)}
          >
            {t("notype.modelExplorer.useModel")}
          </Button>
        )}
        {status === "switching" && <Loader2 className="w-4 h-4 animate-spin" />}
        {(status === "available" || status === "active") && onDelete && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onDelete(model.id)}
            aria-label={t("modelSelector.deleteModel", {
              modelName: displayName,
            })}
          >
            <Trash2 className="w-3.5 h-3.5 inline mr-1" />
            {t("common.delete")}
          </Button>
        )}
      </div>
      {status === "downloading" && (
        <div className="space-y-1" aria-live="polite">
          <progress
            className="w-full accent-logo-primary"
            max={100}
            value={
              downloadProgress === undefined
                ? undefined
                : Math.min(100, Math.max(0, downloadProgress))
            }
            aria-label={t("notype.modelExplorer.downloadProgress")}
          />
          <p className="text-xs tabular-nums">
            {totalBytes !== undefined &&
            totalBytes > 0 &&
            downloadedBytes !== undefined
              ? t("notype.modelExplorer.transferred", {
                  downloaded: formatBytes(downloadedBytes),
                  total: formatBytes(totalBytes),
                })
              : t("notype.modelExplorer.awaitingProgress")}
          </p>
          <div className="flex flex-wrap items-center justify-between gap-2 text-xs">
            <span>
              {t("modelSelector.downloading", {
                percentage: Math.round(downloadProgress ?? 0),
              })}
            </span>
            {downloadSpeed !== undefined && downloadSpeed > 0 && (
              <span>
                {t("modelSelector.downloadSpeed", {
                  speed: downloadSpeed.toFixed(1),
                })}
              </span>
            )}
            {onCancel && (
              <Button
                variant="danger-ghost"
                size="sm"
                onClick={() => onCancel(model.id)}
              >
                {t("modelSelector.cancel")}
              </Button>
            )}
          </div>
        </div>
      )}
      {(status === "verifying" || status === "extracting") && (
        <p className="text-xs" role="status">
          {t(
            status === "verifying"
              ? "modelSelector.verifyingGeneric"
              : "modelSelector.extractingGeneric",
          )}
        </p>
      )}
      <details
        className="model-details"
        onToggle={(event) => {
          setExpanded(event.currentTarget.open);
        }}
      >
        <summary>{t("notype.modelExplorer.advanced")}</summary>
        <div className="space-y-3 pt-3 text-xs leading-relaxed">
          <p>{t("notype.modelExplorer.scoreNote")}</p>
          <div>
            <strong>{t("notype.modelExplorer.languages")}</strong>
            <p className="mt-1">
              {languages.length
                ? [...new Set(model.supported_languages)]
                    .map(
                      (code) => `${getLanguageLabel(code) ?? code} (${code})`,
                    )
                    .join(" · ")
                : t("notype.modelExplorer.unknown")}
            </p>
          </div>
          <p>
            {t(
              model.supports_streaming
                ? "notype.modelExplorer.streamingExplanation"
                : "notype.modelExplorer.batchExplanation",
            )}
          </p>
          <dl className="model-metadata">
            <dt>{t("notype.modelExplorer.location")}</dt>
            <dd>
              {!model.is_downloaded
                ? t("notype.modelExplorer.destinationNote")
                : (modelPath ??
                  t(
                    pathState === "loading"
                      ? "common.loading"
                      : "notype.modelExplorer.pathUnavailable",
                  ))}
            </dd>
            <dt>{t("notype.modelExplorer.filename")}</dt>
            <dd>{model.filename}</dd>
            <dt>{t("notype.modelExplorer.engine")}</dt>
            <dd>{model.engine_type}</dd>
            <dt>{t("notype.modelExplorer.format")}</dt>
            <dd>
              {model.is_directory
                ? t("notype.modelExplorer.directory")
                : (model.filename.split(".").pop()?.toUpperCase() ??
                  t("notype.modelExplorer.unknown"))}
              {quant ? ` / ${quant}` : ""}
            </dd>
            <dt>{t("notype.modelExplorer.source")}</dt>
            <dd>{source}</dd>
          </dl>
          {model.filename === "Qwen3-ASR-0.6B-Q6_K.gguf" && (
            <div className="space-y-1">
              <p>{t("notype.modelExplorer.qwenEvidence")}</p>
              <button
                type="button"
                className="underline underline-offset-2"
                onClick={() => void openQualityReceipt()}
              >
                {t("notype.modelExplorer.qwenReceipt")}
              </button>
              {receiptError && (
                <p role="alert">
                  {t("notype.modelExplorer.qwenReceiptFailed")}
                </p>
              )}
            </div>
          )}
        </div>
      </details>
    </article>
  );
};
export default ModelCard;
