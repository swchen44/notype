import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type ConnectionKind,
  type ConnectionSettingsView,
  type ProviderDialect,
} from "@/bindings";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { CustomWords } from "../CustomWords";
import { ScenarioSettings } from "./ScenarioSettings";
import { ConnectionEditor } from "./ConnectionEditor";

type SettingsResult = Awaited<ReturnType<typeof commands.createTextConnection>>;

export const PostProcessingSettings = () => {
  const { t } = useTranslation();
  const [configuration, setConfiguration] =
    useState<ConnectionSettingsView | null>(null);
  const [busy, setBusy] = useState(false);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<ConnectionKind>("provider");
  const [dialect, setDialect] = useState<ProviderDialect>("generic");
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);
  const [ready, setReady] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editorVersion, setEditorVersion] = useState(0);
  const requestEpoch = useRef(0);

  const apply = (updated: ConnectionSettingsView) => {
    setConfiguration((current) =>
      !current || updated.revision >= current.revision ? updated : current,
    );
  };
  useEffect(() => {
    const epoch = ++requestEpoch.current;
    void Promise.all([
      commands.getTextConfiguration(),
      commands.getWorkflowReadiness(),
    ])
      .then(([result, readiness]) => {
        if (requestEpoch.current !== epoch) return;
        if (result.status === "ok") setConfiguration(result.data);
        else {
          setFailed(true);
          setMessage(t("notype.connections.failed"));
        }
        setReady(readiness.text_configured);
      })
      .catch(() => {
        if (requestEpoch.current !== epoch) return;
        setFailed(true);
        setMessage(t("notype.connections.failed"));
      });
    return () => {
      requestEpoch.current++;
    };
  }, [t]);

  const mutate = async (operation: () => Promise<SettingsResult>) => {
    if (busy) return null;
    const epoch = ++requestEpoch.current;
    setBusy(true);
    setMessage("");
    try {
      const result = await operation();
      if (requestEpoch.current !== epoch) return null;
      if (result.status !== "ok") {
        setFailed(true);
        setMessage(
          t(
            result.error === "configuration-conflict"
              ? "notype.connections.conflict"
              : "notype.connections.failed",
          ),
        );
        return null;
      }
      apply(result.data);
      setCreating(false);
      setName("");
      setFailed(false);
      setMessage(t("notype.connections.saved"));
      const readiness = await commands.getWorkflowReadiness();
      if (requestEpoch.current === epoch) setReady(readiness.text_configured);
      return result.data;
    } catch {
      if (requestEpoch.current !== epoch) return null;
      setFailed(true);
      setMessage(t("notype.connections.failed"));
      return null;
    } finally {
      if (requestEpoch.current === epoch) setBusy(false);
    }
  };

  if (!configuration)
    return (
      <section className="space-y-3">
        <p role={failed ? "alert" : "status"}>
          {message || t("notype.history.loading")}
        </p>
      </section>
    );
  const nameValid =
    name.trim().length > 0 && Array.from(name.trim()).length <= 128;
  const selected = configuration.connections.find(
    (connection) => connection.id === selectedId,
  );
  return (
    <section className="max-w-3xl w-full mx-auto space-y-4">
      <h2 className="font-semibold">{t("notype.connections.title")}</h2>
      <p className="text-sm text-text/70">
        {t("notype.connections.description")}
      </p>
      <p role="status" className="text-sm">
        {t(ready ? "notype.text.ready" : "notype.text.notReady")}
      </p>
      <fieldset disabled={busy} className="space-y-4 min-w-0">
        <Button
          disabled={configuration.connections.length >= 20}
          onClick={() => {
            setCreating((current) => !current);
            setMessage("");
          }}
        >
          {t("notype.connections.add")}
        </Button>
        {creating && (
          <div className="space-y-3 rounded-lg border border-mid-gray/30 p-4">
            <label className="flex flex-col gap-1 text-sm">
              {t("notype.connections.name")}
              <Input
                aria-label={t("notype.connections.name")}
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm">
              {t("notype.connections.kind")}
              <select
                aria-label={t("notype.connections.kind")}
                className="border rounded p-2 bg-background"
                value={kind}
                onChange={(event) =>
                  setKind(event.target.value as ConnectionKind)
                }
              >
                {(["provider", "custom", "local"] as const).map((value) => (
                  <option key={value} value={value}>
                    {t("notype.connections.kinds." + value)}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex flex-col gap-1 text-sm">
              {t("notype.connections.dialect")}
              <select
                aria-label={t("notype.connections.dialect")}
                className="border rounded p-2 bg-background"
                value={dialect}
                onChange={(event) =>
                  setDialect(event.target.value as ProviderDialect)
                }
              >
                {(
                  [
                    "generic",
                    "deepseek",
                    "openrouter",
                    "gemini",
                    "ollama_openai",
                  ] as const
                ).map((value) => (
                  <option key={value} value={value}>
                    {t("notype.connections.dialects." + value)}
                  </option>
                ))}
              </select>
            </label>
            <Button
              disabled={!nameValid}
              onClick={() =>
                void mutate(() =>
                  commands.createTextConnection(
                    configuration.revision,
                    name.trim(),
                    kind,
                    dialect,
                  ),
                )
              }
            >
              {t("notype.connections.create")}
            </Button>
          </div>
        )}
        {configuration.connections.length === 0 ? (
          <p>{t("notype.connections.empty")}</p>
        ) : (
          <ul aria-label={t("notype.connections.list")} className="space-y-3">
            {configuration.connections.map((connection) => (
              <li
                key={connection.id}
                className="rounded-lg border border-mid-gray/30 p-4 space-y-2"
              >
                <p className="font-medium">{connection.display_name}</p>
                <p className="text-sm text-text/70">
                  {t("notype.connections.kinds." + connection.kind)}
                  {" · "}
                  {connection.api_address}
                </p>
                {configuration.active_connection_id === connection.id && (
                  <p role="status">{t("notype.connections.active")}</p>
                )}
                <div className="flex flex-wrap gap-2">
                  <Button
                    variant="secondary"
                    aria-label={t("notype.connections.editNamed", {
                      name: connection.display_name,
                    })}
                    onClick={() => setSelectedId(connection.id)}
                  >
                    {t("notype.connections.edit")}
                  </Button>
                  <Button
                    variant="secondary"
                    disabled={
                      configuration.active_connection_id === connection.id
                    }
                    onClick={() =>
                      void mutate(() =>
                        commands.activateTextConnection(
                          configuration.revision,
                          connection.id,
                        ),
                      )
                    }
                  >
                    {t("notype.connections.activate")}
                  </Button>
                  <Button
                    variant="danger-ghost"
                    onClick={() =>
                      void mutate(() =>
                        commands.removeTextConnection(
                          configuration.revision,
                          connection.id,
                        ),
                      )
                    }
                  >
                    {t("notype.connections.remove")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
        {!configuration.active_connection_id && (
          <p>{t("notype.connections.noActive")}</p>
        )}
      </fieldset>
      {selected && (
        <ConnectionEditor
          key={selected.id + ":" + editorVersion}
          connection={selected}
          revision={configuration.revision}
          busy={busy}
          onSave={async (draft, keyAction) => {
            const updated = await mutate(() =>
              commands.saveTextConnection(
                configuration.revision,
                draft,
                keyAction,
              ),
            );
            if (updated) setEditorVersion((value) => value + 1);
            return !!updated;
          }}
        />
      )}
      {message && <p role={failed ? "alert" : "status"}>{message}</p>}
      <ScenarioSettings configuration={configuration} onChange={apply} />
      <CustomWords descriptionMode="tooltip" />
    </section>
  );
};
