import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import {
  commands,
  type ConnectionDraft,
  type KeyAction,
  type ModelListResult,
} from "@/bindings";
import { Button } from "../../ui/Button";

/** A reference list: neither querying nor copying changes the user's model. */
export function ModelIdList({
  connection,
  keyAction,
  revision,
  disabled,
}: {
  connection: ConnectionDraft;
  keyAction: KeyAction;
  revision: number;
  disabled: boolean;
}) {
  const { t, i18n } = useTranslation();
  const [result, setResult] = useState<ModelListResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);
  const epoch = useRef(0);
  const active = useRef(false);
  const request = useRef<string | null>(null);
  const replacement = keyAction.action === "replace" ? keyAction.value : null;
  const cancelPending = useCallback(() => {
    epoch.current++;
    const id = request.current;
    request.current = null;
    if (id) void commands.cancelTextModels(id).catch(() => undefined);
  }, []);

  useEffect(() => {
    active.current = true;
    cancelPending();
    setResult(null);
    setLoading(false);
    setMessage("");
    return () => {
      active.current = false;
      cancelPending();
    };
  }, [
    connection.id,
    connection.kind,
    connection.dialect,
    connection.address_mode,
    connection.api_address,
    connection.models_endpoint,
    connection.authentication,
    keyAction.action,
    replacement,
    revision,
    disabled,
    cancelPending,
  ]);

  const failureMessage = (code: string | null) => {
    const key = "notype.connections.modelErrors." + (code ?? "");
    return i18n.exists(key) ? t(key) : t("notype.connections.modelsFailed");
  };
  const query = async () => {
    if (loading || request.current || disabled || !active.current) return;
    const id = "models-" + crypto.randomUUID();
    const generation = ++epoch.current;
    request.current = id;
    setLoading(true);
    setResult(null);
    setMessage("");
    setFailed(false);
    try {
      const response = await commands.listTextModels({
        request_id: id,
        expected_revision: revision,
        connection,
        key_action: keyAction,
      });
      if (
        !active.current ||
        epoch.current !== generation ||
        request.current !== id
      )
        return;
      request.current = null;
      if (response.status !== "ok") {
        setFailed(true);
        setMessage(failureMessage(response.error));
        return;
      }
      const data = response.data;
      if (
        data.request_id !== id ||
        data.configuration_revision !== revision ||
        data.connection_id !== connection.id
      ) {
        setFailed(true);
        setMessage(t("notype.connections.modelsStale"));
        return;
      }
      setResult(data);
      setFailed(!["ok", "cancelled"].includes(data.status));
      switch (data.status) {
        case "ok":
          setMessage(
            t(
              data.models.length
                ? "notype.connections.modelsReady"
                : "notype.connections.modelsEmpty",
            ),
          );
          break;
        case "unsupported":
          setMessage(t("notype.connections.modelsUnsupported"));
          break;
        case "cancelled":
          setMessage(t("notype.connections.modelsCancelled"));
          break;
        case "throttled":
          setMessage(t("notype.connections.modelsThrottled"));
          break;
        case "stale":
          setMessage(t("notype.connections.modelsStale"));
          break;
        default:
          setMessage(failureMessage(data.error_code));
      }
    } catch {
      if (!active.current || epoch.current !== generation) return;
      setFailed(true);
      setMessage(t("notype.connections.modelsFailed"));
    } finally {
      if (request.current === id) request.current = null;
      if (active.current && epoch.current === generation) setLoading(false);
    }
  };
  const copy = async (id: string) => {
    const generation = epoch.current;
    try {
      await writeText(id);
      if (active.current && epoch.current === generation) {
        setFailed(false);
        setMessage(t("notype.connections.modelCopied"));
      }
    } catch {
      if (active.current && epoch.current === generation) {
        setFailed(true);
        setMessage(t("notype.connections.copyFailed"));
      }
    }
  };
  const listUnavailable =
    connection.address_mode === "full_chat_endpoint" &&
    !connection.models_endpoint;
  return (
    <section
      aria-label={t("notype.connections.modelReference")}
      className="space-y-3"
    >
      <p className="text-sm text-text/70">
        {t("notype.connections.modelsHint")}
      </p>
      <div className="flex flex-wrap gap-2">
        <Button
          variant="secondary"
          disabled={disabled || loading || listUnavailable}
          onClick={() => void query()}
        >
          {t(
            loading
              ? "notype.connections.modelsLoading"
              : "notype.connections.viewModels",
          )}
        </Button>
        {loading && (
          <Button
            variant="secondary"
            onClick={() => {
              cancelPending();
              setLoading(false);
              setResult(null);
              setFailed(false);
              setMessage(t("notype.connections.modelsCancelled"));
            }}
          >
            {t("notype.connections.cancelModels")}
          </Button>
        )}
      </div>
      {listUnavailable && (
        <p role="note" className="text-sm">
          {t("notype.connections.modelsEndpointRequired")}
        </p>
      )}
      {message && (
        <p role={failed ? "alert" : "status"} className="text-sm">
          {message}
        </p>
      )}
      {result && (
        <p className="text-sm">
          {t("notype.connections.queryKeySource")}:{" "}
          {t("notype.connections.sources." + result.credential_source)}
        </p>
      )}
      {result?.retry_after_seconds !== null &&
        result?.retry_after_seconds !== undefined &&
        result.retry_after_seconds > 0 && (
          <p className="text-sm">
            {t("notype.connections.retryAfter", {
              seconds: Math.min(86400, Math.ceil(result.retry_after_seconds)),
            })}
          </p>
        )}
      {result?.status === "ok" && result.models.length > 0 && (
        <ul
          aria-label={t("notype.connections.modelList")}
          className="max-h-64 overflow-y-auto space-y-2 border border-mid-gray/30 rounded-lg p-3"
        >
          {result.models.map((model) => (
            <li
              key={model.id}
              className="flex flex-wrap items-center justify-between gap-2"
            >
              <code className="break-all select-text">{model.id}</code>
              <Button
                variant="secondary"
                size="sm"
                aria-label={t("notype.connections.copyModel", { id: model.id })}
                onClick={() => void copy(model.id)}
              >
                {t("notype.connections.copyId")}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
