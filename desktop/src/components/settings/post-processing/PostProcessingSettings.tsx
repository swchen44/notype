import { useCallback, useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type ConnectionKind,
  type ConnectionSettingsView,
  type ProviderDialect,
} from "@/bindings";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { ScenarioSettings } from "./ScenarioSettings";
import { ConnectionEditor } from "./ConnectionEditor";
import { PostProcessingToggle } from "../PostProcessingToggle";
import { ConnectionActivity } from "./ConnectionActivity";
import {
  connectionConfigurationChanged,
  useConnectionActivity,
} from "@/hooks/useConnectionActivity";
import { useSettings } from "@/hooks/useSettings";

type SettingsResult = Awaited<ReturnType<typeof commands.createTextConnection>>;

export const PostProcessingSettings = () => {
  const { t } = useTranslation();
  const tabId = useId();
  const [tab, setTab] = useState<"servers" | "prompts">("servers");
  const { activity, refresh: refreshActivity } = useConnectionActivity();
  const { getSetting } = useSettings();
  const enabled = getSetting("post_process_enabled") ?? false;
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
  const [editorVisit, setEditorVisit] = useState(0);
  const editorAnchor = useRef<HTMLDivElement>(null);
  const openEditor = (id: string) => {
    setTab("servers");
    setSelectedId(id);
    setEditorVisit((value) => value + 1);
  };
  useEffect(() => {
    if (!selectedId) return;
    editorAnchor.current?.scrollIntoView({ block: "start" });
    editorAnchor.current
      ?.querySelector<HTMLInputElement>("input")
      ?.focus({ preventScroll: true });
  }, [selectedId, editorVisit]);
  const [reloadVersion, setReloadVersion] = useState(0);
  const requestEpoch = useRef(0);
  const readinessEpoch = useRef(0);
  const mutationLock = useRef(false);

  const apply = useCallback((updated: ConnectionSettingsView) => {
    setConfiguration((current) =>
      !current || updated.revision >= current.revision ? updated : current,
    );
    connectionConfigurationChanged();
  }, []);
  useEffect(() => {
    const epoch = ++requestEpoch.current;
    setBusy(true);
    setMessage("");
    setFailed(false);
    void commands
      .getTextConfiguration()
      .then((result) => {
        if (requestEpoch.current !== epoch) return;
        if (result.status === "ok") apply(result.data);
        else {
          setFailed(true);
          setMessage(t("notype.connections.failed"));
        }
      })
      .catch(() => {
        if (requestEpoch.current !== epoch) return;
        setFailed(true);
        setMessage(t("notype.connections.failed"));
      })
      .finally(() => {
        if (requestEpoch.current === epoch) setBusy(false);
      });
    return () => {
      requestEpoch.current++;
    };
  }, [t, reloadVersion, apply]);

  // The master switch changes readiness without changing saved connections or drafts.
  useEffect(() => {
    const epoch = ++readinessEpoch.current;
    setReady(false);
    if (enabled) {
      void commands
        .getWorkflowReadiness()
        .then((readiness) => {
          if (readinessEpoch.current === epoch)
            setReady(readiness.text_configured);
        })
        .catch(() => {
          if (readinessEpoch.current === epoch) setReady(false);
        });
    }
    return () => {
      readinessEpoch.current++;
    };
  }, [enabled, configuration?.revision, reloadVersion]);

  const mutate = async (operation: () => Promise<SettingsResult>) => {
    if (busy || mutationLock.current) return null;
    mutationLock.current = true;
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
              : result.error === "prompt-tuning-invalid" ||
                  result.error === "prompt-tuning-too-long"
                ? "notype.tuning.invalid"
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
      return result.data;
    } catch {
      if (requestEpoch.current !== epoch) return null;
      setFailed(true);
      setMessage(t("notype.connections.failed"));
      return null;
    } finally {
      mutationLock.current = false;
      if (requestEpoch.current === epoch) setBusy(false);
    }
  };

  if (!configuration)
    return (
      <section className="space-y-3">
        <p role={failed ? "alert" : "status"}>
          {message || t("notype.history.loading")}
        </p>
        <Button
          disabled={busy}
          onClick={() => setReloadVersion((value) => value + 1)}
        >
          {t("notype.connections.reload")}
        </Button>
      </section>
    );
  const nameValid =
    name.trim().length > 0 && Array.from(name.trim()).length <= 128;
  const selected = configuration.connections.find(
    (connection) => connection.id === selectedId,
  );
  return (
    <section className="max-w-3xl w-full mx-auto space-y-4 break-words">
      <h2 className="font-semibold">{t("notype.connections.title")}</h2>
      <PostProcessingToggle descriptionMode="inline" />
      <p className="text-sm text-text/70">
        {t("notype.connections.description")}
      </p>
      <p role="status" className="text-sm">
        {t(
          !enabled
            ? "notype.usability.aiOff"
            : ready
              ? "notype.text.ready"
              : "notype.text.notReady",
        )}
      </p>
      <div
        role="tablist"
        aria-label={t("notype.settingsTabs.title")}
        className="flex border-b border-mid-gray/30 gap-2"
      >
        {(["servers", "prompts"] as const).map((id) => (
          <button
            key={id}
            type="button"
            role="tab"
            id={`${tabId}-${id}-tab`}
            aria-controls={`${tabId}-${id}-panel`}
            aria-selected={tab === id}
            tabIndex={tab === id ? 0 : -1}
            className={`px-4 py-2 border-b-2 ${tab === id ? "border-text font-medium" : "border-transparent text-text/60"}`}
            onClick={() => setTab(id)}
            onKeyDown={(e) => {
              if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
                e.preventDefault();
                const next =
                  e.key === "Home"
                    ? "servers"
                    : e.key === "End"
                      ? "prompts"
                      : id === "servers"
                        ? "prompts"
                        : "servers";
                setTab(next);
                document.getElementById(`${tabId}-${next}-tab`)?.focus();
              }
            }}
          >
            {t(`notype.settingsTabs.${id}`)}
          </button>
        ))}
      </div>
      <div
        role="tabpanel"
        id={`${tabId}-servers-panel`}
        aria-labelledby={`${tabId}-servers-tab`}
        hidden={tab !== "servers"}
        className="space-y-4"
      >
        <fieldset disabled={busy} className="space-y-4 min-w-0">
          <Button
            variant="secondary"
            onClick={() => {
              setSelectedId(null);
              setCreating(false);
              setReloadVersion((value) => value + 1);
            }}
          >
            {t("notype.connections.reload")}
          </Button>
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
              <p className="text-sm text-text/70">
                {t("notype.connections.createHint")}
              </p>
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
                onClick={async () => {
                  const updated = await mutate(() =>
                    commands.createTextConnection(
                      configuration.revision,
                      name.trim(),
                      kind,
                      dialect,
                    ),
                  );
                  const created = updated?.connections.find(
                    (connection) =>
                      !configuration.connections.some(
                        (saved) => saved.id === connection.id,
                      ),
                  );
                  if (created) openEditor(created.id);
                }}
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
                  {connection.masked_api_key && (
                    <p className="font-mono text-sm">
                      {t("notype.usability.savedKey")}:{" "}
                      {connection.masked_api_key}
                    </p>
                  )}
                  <label className="flex gap-2 items-center text-sm">
                    <input
                      type="radio"
                      name="active-text-connection"
                      aria-label={t("notype.usability.selectConnection", {
                        name: connection.display_name,
                      })}
                      checked={
                        configuration.active_connection_id === connection.id
                      }
                      onChange={() =>
                        void mutate(() =>
                          commands.activateTextConnection(
                            configuration.revision,
                            connection.id,
                          ),
                        )
                      }
                    />
                    {t("notype.usability.useConnection")}
                  </label>
                  <ConnectionActivity
                    id={connection.id}
                    revision={configuration.revision}
                    activity={activity.find(
                      (item) => item.connection_id === connection.id,
                    )}
                    refresh={refreshActivity}
                  />
                  <div className="flex flex-wrap gap-2">
                    <Button
                      variant="secondary"
                      aria-label={t("notype.connections.editNamed", {
                        name: connection.display_name,
                      })}
                      onClick={() => openEditor(connection.id)}
                    >
                      {t("notype.connections.edit")}
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
          <div ref={editorAnchor} className="scroll-mt-4">
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
          </div>
        )}
        {message && <p role={failed ? "alert" : "status"}>{message}</p>}
      </div>
      <div
        role="tabpanel"
        id={`${tabId}-prompts-panel`}
        aria-labelledby={`${tabId}-prompts-tab`}
        hidden={tab !== "prompts"}
      >
        <ScenarioSettings
          configuration={configuration}
          onChange={apply}
          disabled={busy}
        />
      </div>
    </section>
  );
};
