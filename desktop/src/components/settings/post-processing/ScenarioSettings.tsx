import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type SceneId, type TextConfiguration } from "@/bindings";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

export function ScenarioSettings({
  configuration,
  onChange,
}: {
  configuration: TextConfiguration;
  onChange: (configuration: TextConfiguration) => void;
}) {
  const { t } = useTranslation();
  const [scene, setScene] = useState<SceneId>("dictation");
  const [prompt, setPrompt] = useState(configuration.prompts.dictation);
  const [translation, setTranslation] = useState(configuration.translation);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState(false);
  const run = async (
    operation: () => Promise<
      | { status: "ok"; data: TextConfiguration }
      | { status: "error"; error: string }
    >,
    reset = false,
  ) => {
    setBusy(true);
    setMessage("");
    try {
      const result = await operation();
      if (result.status !== "ok") throw new Error(result.error);
      onChange(result.data);
      if (reset) setPrompt(result.data.prompts[scene]);
      setError(false);
      setMessage(
        t(reset ? "notype.scenarios.restored" : "notype.scenarios.saved"),
      );
    } catch {
      setError(true);
      setMessage(t("notype.scenarios.invalid"));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section
      className="space-y-4 border-t border-mid-gray/30 pt-4"
      aria-label={t("notype.scenarios.title")}
    >
      <h3 className="font-semibold">{t("notype.scenarios.title")}</h3>
      <p className="text-sm text-text/70">{t("notype.scenarios.snapshot")}</p>
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.scenarios.scene")}
        <select
          aria-label={t("notype.scenarios.scene")}
          className="border rounded p-2 bg-background"
          value={scene}
          disabled={busy}
          onChange={(event) => {
            const next = event.target.value as SceneId;
            setScene(next);
            setPrompt(configuration.prompts[next]);
            setMessage("");
          }}
        >
          {(["dictation", "translation", "ask"] as const).map((id) => (
            <option key={id} value={id}>
              {t(`notype.scenarios.${id}`)}
            </option>
          ))}
        </select>
      </label>
      <label className="flex flex-col gap-1 text-sm">
        {t("notype.scenarios.prompt")}
        <textarea
          aria-label={t("notype.scenarios.prompt")}
          className="border rounded p-2 bg-background min-h-64 font-mono text-sm"
          value={prompt}
          disabled={busy}
          onChange={(event) => {
            setPrompt(event.target.value);
            setMessage("");
          }}
          spellCheck={false}
        />
      </label>
      <div className="flex gap-2">
        <Button
          disabled={busy || !prompt.trim()}
          onClick={() =>
            void run(() =>
              commands.saveScenarioPrompt(
                scene,
                prompt,
                configuration.revision,
              ),
            )
          }
        >
          {t("notype.scenarios.savePrompt")}
        </Button>
        <Button
          variant="secondary"
          disabled={busy}
          onClick={() =>
            void run(
              () => commands.resetScenarioPrompt(scene, configuration.revision),
              true,
            )
          }
        >
          {t("notype.scenarios.resetPrompt")}
        </Button>
      </div>
      <p className="text-sm text-text/70">{t("notype.scenarios.resetScope")}</p>
      {scene === "translation" && (
        <div className="space-y-3">
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.scenarios.target")}
            <Input
              aria-label={t("notype.scenarios.target")}
              value={translation.target_language}
              onChange={(event) =>
                setTranslation({
                  ...translation,
                  target_language: event.target.value,
                })
              }
            />
          </label>
          <p className="text-sm text-text/70">
            {t("notype.scenarios.targetHint")}
          </p>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.scenarios.domain")}
            <Input
              aria-label={t("notype.scenarios.domain")}
              value={translation.domain}
              onChange={(event) =>
                setTranslation({ ...translation, domain: event.target.value })
              }
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            {t("notype.scenarios.tone")}
            <Input
              aria-label={t("notype.scenarios.tone")}
              value={translation.tone}
              onChange={(event) =>
                setTranslation({ ...translation, tone: event.target.value })
              }
            />
          </label>
          <h4 className="text-sm font-semibold">
            {t("notype.scenarios.mappings")}
          </h4>
          {translation.glossary.map((term, index) => (
            <div key={index} className="flex flex-wrap gap-2">
              {(["source", "target", "target_language"] as const).map(
                (field) => (
                  <Input
                    key={field}
                    aria-label={`${t(`notype.scenarios.mapping_${field}`)} ${index + 1}`}
                    value={term[field]}
                    onChange={(event) =>
                      setTranslation({
                        ...translation,
                        glossary: translation.glossary.map((item, i) =>
                          i === index
                            ? { ...item, [field]: event.target.value }
                            : item,
                        ),
                      })
                    }
                  />
                ),
              )}
              <Button
                variant="secondary"
                onClick={() =>
                  setTranslation({
                    ...translation,
                    glossary: translation.glossary.filter(
                      (_, i) => i !== index,
                    ),
                  })
                }
              >
                {t("notype.scenarios.removeMapping")}
              </Button>
            </div>
          ))}
          <Button
            variant="secondary"
            onClick={() =>
              setTranslation({
                ...translation,
                glossary: [
                  ...translation.glossary,
                  {
                    source: "",
                    target: "",
                    target_language: translation.target_language,
                  },
                ],
              })
            }
          >
            {t("notype.scenarios.addMapping")}
          </Button>
          <label className="flex gap-2 text-sm">
            <input
              aria-label={t("notype.scenarios.backTranslation")}
              type="checkbox"
              checked={translation.back_translation}
              onChange={(event) =>
                setTranslation({
                  ...translation,
                  back_translation: event.target.checked,
                })
              }
            />
            {t("notype.scenarios.backTranslation")}
          </label>
          <p className="text-sm text-text/70">
            {t("notype.scenarios.backHint")}
          </p>
          <Button
            disabled={busy || !translation.target_language.trim()}
            onClick={() =>
              void run(() =>
                commands.saveTranslationSettings(
                  translation,
                  configuration.revision,
                ),
              )
            }
          >
            {t("notype.scenarios.saveTranslation")}
          </Button>
        </div>
      )}
      {message && (
        <p role={error ? "alert" : "status"} className="text-sm">
          {message}
        </p>
      )}
    </section>
  );
}
