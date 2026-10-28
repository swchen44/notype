import { invoke } from "@tauri-apps/api/core";

// Only sanitized backend previews cross IPC. Archive/key payloads stay native.
export type SettingsImportPreview = {
  id: string;
  before_yaml: string;
  after_yaml: string;
  changed_fields: string[];
  key_effects: { name: string; effect: string }[];
  conflicts: string[];
};
export const settingsTransfer = {
  export: (includeKeys: boolean) =>
    invoke<{ status: string }>("export_settings_yaml", { includeKeys }),
  preview: () =>
    invoke<SettingsImportPreview | null>("preview_settings_import"),
  resolveFolder: (id: string) =>
    invoke<SettingsImportPreview | null>("resolve_settings_import_folder", {
      id,
    }),
  resolveAudio: (id: string) =>
    invoke<SettingsImportPreview>("resolve_settings_import_audio", { id }),
  cancel: (id: string) => invoke<void>("cancel_settings_import", { id }),
  apply: (id: string) =>
    invoke<{ status: string }>("apply_settings_import", { id }),
};
