import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands, type TextConfiguration } from "@/bindings";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { CustomWords } from "../CustomWords";
import { ScenarioSettings } from "./ScenarioSettings";

export const PostProcessingSettings = () => {
  const { t } = useTranslation();
  const [configuration, setConfiguration] = useState<TextConfiguration | null>(
    null,
  );
  const [saving, setSaving] = useState(false);
  const [ready, setReady] = useState(false);
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    let active = true;
    void Promise.all([
      commands.getTextConfiguration(),
      commands.getWorkflowReadiness(),
    ])
      .then(([result, readiness]) => {
        if (!active) return;
        if (result.status === "ok") setConfiguration(result.data);
        else toast.error(t("notype.history.actionFailed"));
        setReady(readiness.text_configured);
      })
      .catch(() => {
        if (active) toast.error(t("notype.history.actionFailed"));
      });
    return () => {
      active = false;
    };
  }, [t]);
  if (!configuration) return <p>{t("notype.history.loading")}</p>;
  const edit = (value: Partial<TextConfiguration>) => {
    setConfiguration({ ...configuration, ...value });
    setReady(false);
    setSaved(false);
  };
  const save = async () => {
    setSaving(true);
    try {
      const result = await commands.setTextConfiguration(configuration);
      if (result.status !== "ok")
        throw new Error("configuration-storage-failed");
      setReady((await commands.getWorkflowReadiness()).text_configured);
      setSaved(true);
    } catch {
      toast.error(t("notype.history.actionFailed"));
    } finally {
      setSaving(false);
    }
  };
  return (
    <section className="max-w-3xl w-full mx-auto space-y-4">
      <h2 className="font-semibold">{t("notype.text.title")}</h2>
      <p className="text-sm text-text/70">{t("notype.text.description")}</p>
      <p className="text-sm">
        {t(ready ? "notype.text.ready" : "notype.text.notReady")}
      </p>
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.text.transport")}
        <select
          aria-label={t("notype.text.transport")}
          className="border rounded p-2 bg-background"
          value={configuration.transport}
          onChange={(event) =>
            edit({
              transport: event.target.value as TextConfiguration["transport"],
            })
          }
        >
          <option value="disabled">{t("notype.text.disabled")}</option>
          <option value="open_ai_chat">{t("notype.text.chat")}</option>
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.text.endpoint")}
        <Input
          aria-label={t("notype.text.endpoint")}
          value={configuration.endpoint}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => edit({ endpoint: event.target.value })}
        />
      </label>
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.text.model")}
        <Input
          aria-label={t("notype.text.model")}
          value={configuration.model}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => edit({ model: event.target.value })}
        />
      </label>
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.text.authentication")}
        <select
          aria-label={t("notype.text.authentication")}
          className="border rounded p-2 bg-background"
          value={configuration.authentication}
          onChange={(event) =>
            edit({
              authentication: event.target
                .value as TextConfiguration["authentication"],
            })
          }
        >
          <option value="unspecified">
            {t("notype.text.chooseAuthentication")}
          </option>
          <option value="none">{t("notype.text.noAuth")}</option>
          <option value="bearer">{t("notype.text.bearer")}</option>
        </select>
      </label>
      {configuration.authentication === "bearer" && (
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.key")}
          <Input
            aria-label={t("notype.text.key")}
            type="password"
            autoComplete="new-password"
            value={configuration.api_key}
            onChange={(event) => edit({ api_key: event.target.value })}
          />
        </label>
      )}
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.text.timeout")}
        <Input
          aria-label={t("notype.text.timeout")}
          type="number"
          min={1}
          max={120}
          value={configuration.timeout_ms / 1000}
          onChange={(event) =>
            edit({ timeout_ms: Number(event.target.value) * 1000 })
          }
        />
      </label>
      <Button
        onClick={() => {
          void save();
        }}
        disabled={saving}
      >
        {t("notype.text.save")}
      </Button>
      {saved && (
        <p role="status" className="text-sm">
          {t("notype.text.saved")}
        </p>
      )}
      <p className="text-sm text-text/70">{t("notype.text.preset")}</p>
      <ScenarioSettings
        configuration={configuration}
        onChange={(updated) =>
          setConfiguration({
            ...configuration,
            prompts: updated.prompts,
            translation: updated.translation,
          })
        }
      />
      <CustomWords descriptionMode="tooltip" />
    </section>
  );
};
