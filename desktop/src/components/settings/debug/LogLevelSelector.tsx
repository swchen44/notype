import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../../ui/SettingContainer";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";
import { useSettings } from "../../../hooks/useSettings";
import type { LogLevel } from "../../../bindings";

const LOG_LEVEL_OPTIONS: DropdownOption[] = [
  { value: "error", label: "Error" },
  { value: "warn", label: "Warn" },
  { value: "info", label: "Info" },
  { value: "debug", label: "Debug" },
  { value: "trace", label: "Trace" },
];

interface LogLevelSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const LogLevelSelector: React.FC<LogLevelSelectorProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const currentLevel = settings?.log_level ?? "debug";

  const handleSelect = async (value: string) => {
    if (value === currentLevel) return;

    setSaving(true);
    setError("");
    try {
      await invoke<void>("set_log_level", { level: value as LogLevel });
      await refreshSettings();
    } catch {
      setError(t("notype.diagnostics.logLevelFailed"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <SettingContainer
      title={t("settings.debug.logLevel.title")}
      description={t("notype.diagnostics.logLevelDescription")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
    >
      <div>
        <Dropdown
          options={LOG_LEVEL_OPTIONS}
          selectedValue={currentLevel}
          onSelect={handleSelect}
          disabled={!settings || saving}
        />
        <p role="status" className="text-xs">
          {error}
        </p>
      </div>
    </SettingContainer>
  );
};
