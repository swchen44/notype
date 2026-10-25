import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import type { RecognitionModeSetting } from "../../bindings";
import { SettingContainer } from "../ui/SettingContainer";

export function RecognitionModeSelector() {
  const { t } = useTranslation();
  const { settings, updateSetting, isUpdating } = useSettings();
  return (
    <SettingContainer
      title={t("notype.recognition.mode")}
      description={t("notype.recognition.description")}
      grouped
    >
      <select
        aria-label={t("notype.recognition.mode")}
        value={settings?.recognition_mode ?? "traditional_english"}
        disabled={isUpdating("recognition_mode")}
        onChange={(event) =>
          void updateSetting(
            "recognition_mode",
            event.target.value as RecognitionModeSetting,
          )
        }
        className="rounded-md border border-mid-gray/80 bg-mid-gray/10 px-2 py-1 text-sm text-text"
      >
        <option value="traditional_english">
          {t("notype.recognition.traditional")}
        </option>
        <option value="simplified_english">
          {t("notype.recognition.simplified")}
        </option>
      </select>
    </SettingContainer>
  );
}
