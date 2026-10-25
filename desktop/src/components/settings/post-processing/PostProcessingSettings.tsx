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
  const [customEffortMode, setCustomEffortMode] = useState(false);
  const [maxOutput, setMaxOutput] = useState("");
  const [saveFailed, setSaveFailed] = useState(false);
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
        if (result.status === "ok") {
          setConfiguration(result.data);
          setMaxOutput(result.data.max_output_tokens?.toString() ?? "");
        } else toast.error(t("notype.history.actionFailed"));
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
    setSaveFailed(false);
  };
  const maximum =
    configuration.provider_profile === "deepseek" ? 393216 : 1000000;
  const effortValid =
    configuration.effort === null ||
    (configuration.provider_profile === "deepseek"
      ? ["none", "low", "high", "max"].includes(configuration.effort)
      : /^[a-z0-9_-]{1,32}$/.test(configuration.effort));
  const limitValid =
    maxOutput === "" ||
    (/^[0-9]+$/.test(maxOutput) &&
      Number.isInteger(Number(maxOutput)) &&
      Number(maxOutput) >= 1 &&
      Number(maxOutput) <= maximum);
  const standardEfforts =
    configuration.provider_profile === "deepseek"
      ? ["none", "low", "high", "max"]
      : ["none", "minimal", "low", "medium", "high", "xhigh", "max"];
  const customEffort =
    configuration.provider_profile === "generic" &&
    configuration.effort !== null &&
    (customEffortMode || !standardEfforts.includes(configuration.effort));
  const save = async () => {
    if (!effortValid || !limitValid || saving) return;
    setSaving(true);
    setSaved(false);
    setSaveFailed(false);
    try {
      const result = await commands.setTextConfiguration({
        ...configuration,
        max_output_tokens: maxOutput === "" ? null : Number(maxOutput),
      });
      if (result.status !== "ok")
        throw new Error("configuration-storage-failed");
      setReady((await commands.getWorkflowReadiness()).text_configured);
      setSaved(true);
    } catch {
      setSaveFailed(true);
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
      <fieldset disabled={saving} className="space-y-4 min-w-0">
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
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.providerProfile")}
          <select
            aria-label={t("notype.text.providerProfile")}
            className="border rounded p-2 bg-background min-w-0"
            value={configuration.provider_profile}
            onChange={(event) =>
              edit({
                provider_profile: event.target
                  .value as TextConfiguration["provider_profile"],
              })
            }
          >
            <option value="generic">{t("notype.text.generic")}</option>
            <option value="deepseek">{t("notype.text.deepseek")}</option>
          </select>
        </label>
        <p className="text-sm text-text/70">{t("notype.text.optionHint")}</p>
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.effort")}
          <select
            aria-label={t("notype.text.effort")}
            aria-invalid={!effortValid}
            aria-describedby={!effortValid ? "effort-error" : undefined}
            className="border rounded p-2 bg-background min-w-0"
            value={
              configuration.effort === null
                ? "default"
                : customEffort
                  ? "custom"
                  : configuration.effort
            }
            onChange={(event) => {
              setCustomEffortMode(event.target.value === "custom");
              edit({
                effort:
                  event.target.value === "default"
                    ? null
                    : event.target.value === "custom"
                      ? ""
                      : event.target.value,
              });
            }}
          >
            <option value="default">
              {t("notype.metrics.providerDefault")}
            </option>
            {standardEfforts.map((effort) => (
              <option key={effort} value={effort}>
                {effort}
              </option>
            ))}
            {configuration.provider_profile === "generic" && (
              <option value="custom">{t("notype.text.customEffort")}</option>
            )}
            {configuration.provider_profile === "deepseek" && !effortValid && (
              <option value={configuration.effort ?? ""}>
                {configuration.effort}
              </option>
            )}
          </select>
        </label>
        {customEffort && (
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.text.customEffort")}
            <Input
              aria-label={t("notype.text.customEffort")}
              aria-invalid={!effortValid}
              aria-describedby={!effortValid ? "effort-error" : undefined}
              autoComplete="off"
              spellCheck={false}
              value={configuration.effort ?? ""}
              onChange={(event) => edit({ effort: event.target.value })}
            />
          </label>
        )}
        {!effortValid && (
          <p id="effort-error" role="alert" className="text-sm">
            {t(
              configuration.provider_profile === "deepseek"
                ? "notype.text.deepseekEffortInvalid"
                : "notype.text.genericEffortInvalid",
            )}
          </p>
        )}
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.maxOutput")}
          <Input
            aria-label={t("notype.text.maxOutput")}
            aria-invalid={!limitValid}
            aria-describedby={!limitValid ? "max-output-error" : undefined}
            inputMode="numeric"
            value={maxOutput}
            onChange={(event) => {
              setMaxOutput(event.target.value);
              setSaved(false);
              setSaveFailed(false);
              setReady(false);
            }}
          />
        </label>
        {!limitValid && (
          <p id="max-output-error" role="alert" className="text-sm">
            {t("notype.text.limitInvalid", { maximum })}
          </p>
        )}
        <Button
          onClick={() => {
            void save();
          }}
          disabled={saving}
        >
          {t("notype.text.save")}
        </Button>
      </fieldset>
      {saveFailed && (
        <p role="status" className="text-sm">
          {t("notype.text.saveFailed")}
        </p>
      )}
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
