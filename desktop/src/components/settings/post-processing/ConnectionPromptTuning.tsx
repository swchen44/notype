import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ConnectionPromptTuning as Tuning, SceneId } from "@/bindings";

/** Edits only the selected connection draft; saving remains one explicit action. */
export function ConnectionPromptTuning({
  value,
  onChange,
}: {
  value: Tuning;
  onChange: (value: Tuning) => void;
}) {
  const { t } = useTranslation();
  const [scene, setScene] = useState<SceneId>("dictation");
  return (
    <fieldset className="space-y-3 rounded-lg border border-mid-gray/30 p-3 min-w-0">
      <legend className="text-sm font-medium">
        {t("notype.tuning.title")}
      </legend>
      <p className="text-sm text-text/70">{t("notype.tuning.hint")}</p>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.tuning.scene")}
          <select
            aria-label={t("notype.tuning.scene")}
            className="border rounded p-2 bg-background"
            value={scene}
            onChange={(e) => setScene(e.target.value as SceneId)}
          >
            {(["dictation", "translation", "ask"] as const).map((id) => (
              <option key={id} value={id}>
                {t(`notype.scenarios.${id}`)}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.tuning.mode")}
          <select
            aria-label={t("notype.tuning.mode")}
            className="border rounded p-2 bg-background"
            value={value[scene] === null ? "none" : "custom"}
            onChange={(e) =>
              onChange({
                ...value,
                [scene]: e.target.value === "none" ? null : "",
              })
            }
          >
            <option value="none">{t("notype.tuning.none")}</option>
            <option value="custom">{t("notype.tuning.custom")}</option>
          </select>
        </label>
      </div>
      {value[scene] !== null && (
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.tuning.rules")}
          <textarea
            aria-label={t("notype.tuning.rules")}
            className="border rounded p-2 bg-background min-h-32 font-mono text-sm"
            value={value[scene] ?? ""}
            spellCheck={false}
            onChange={(e) => onChange({ ...value, [scene]: e.target.value })}
          />
        </label>
      )}
    </fieldset>
  );
}
