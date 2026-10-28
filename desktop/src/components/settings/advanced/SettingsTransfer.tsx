import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  settingsTransfer,
  type SettingsImportPreview,
} from "../../../lib/settingsTransfer";
import { Button } from "../../ui/Button";
import { SettingsGroup } from "../../ui/SettingsGroup";

export function SettingsTransfer() {
  const { t } = useTranslation();
  const [includeKeys, setIncludeKeys] = useState(false);
  const [preview, setPreview] = useState<SettingsImportPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [restarting, setRestarting] = useState(false);
  const tr = (key: string) => t(`settings.transfer.${key}`);
  async function run(task: () => Promise<void>) {
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await task();
    } catch (error) {
      const code = String(error).replace(/^Error: /, "");
      const category =
        code === "settings-transfer-stale"
          ? "stale"
          : /busy|locked/.test(code)
            ? "busy"
            : /yaml|invalid/.test(code)
              ? "invalid"
              : /space/.test(code)
                ? "space"
                : "failed";
      setError(tr(`errors.${category}`));
    } finally {
      setBusy(false);
    }
  }
  return (
    <SettingsGroup title={tr("title")}>
      <div className="p-4 space-y-4 min-w-0">
        <p className="text-sm text-text/70">{tr("scope")}</p>
        <label className="flex items-start gap-2 text-sm">
          <input
            type="checkbox"
            checked={includeKeys}
            disabled={busy || restarting}
            onChange={(event) => setIncludeKeys(event.target.checked)}
          />
          <span>{tr("includeKeys")}</span>
        </label>
        {includeKeys && (
          <p className="text-sm text-warning" role="note">
            {tr("plaintext")}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            disabled={busy || restarting || !!preview}
            onClick={() =>
              void run(async () => {
                const result = await settingsTransfer.export(includeKeys);
                setNotice(
                  tr(result.status === "saved" ? "saved" : "cancelled"),
                );
              })
            }
          >
            {tr("export")}
          </Button>
          <Button
            variant="secondary"
            disabled={busy || restarting || !!preview}
            onClick={() =>
              void run(async () => {
                const result = await settingsTransfer.preview();
                setPreview(result);
                if (!result) setNotice(tr("cancelled"));
              })
            }
          >
            {tr("import")}
          </Button>
        </div>
        {notice && <p role="status">{notice}</p>}
        {error && (
          <p role="alert" className="text-warning">
            {error}
          </p>
        )}
        {preview && (
          <section
            aria-label={tr("preview")}
            className="space-y-4 border border-mid-gray/30 rounded-xl p-4 min-w-0"
          >
            <h3 className="font-semibold">{tr("preview")}</h3>
            <p className="text-sm">{tr("replace")}</p>
            <div>
              <h4 className="font-medium">{tr("changes")}</h4>
              {preview.changed_fields.length ? (
                <ul className="list-disc pl-5 text-sm break-all">
                  {preview.changed_fields.map((field) => (
                    <li key={field}>{field}</li>
                  ))}
                </ul>
              ) : (
                <p className="text-sm">{tr("noChanges")}</p>
              )}
            </div>
            <div className="grid grid-cols-1 xl:grid-cols-2 gap-3 min-w-0">
              {(["before", "after"] as const).map((side) => (
                <div key={side} className="min-w-0">
                  <h4 className="font-medium">{tr(side)}</h4>
                  <pre className="text-xs whitespace-pre-wrap break-all max-h-64 overflow-auto rounded border border-mid-gray/20 p-2">
                    {side === "before"
                      ? preview.before_yaml
                      : preview.after_yaml}
                  </pre>
                </div>
              ))}
            </div>
            <div>
              <h4 className="font-medium">{tr("keys")}</h4>
              <ul className="text-sm space-y-1">
                {preview.key_effects.map((item, index) => (
                  <li key={index}>
                    {item.name}: {tr(`keyEffects.${item.effect}`)}
                  </li>
                ))}
              </ul>
              <p className="text-sm text-text/70">{tr("omittedKeys")}</p>
            </div>
            {preview.conflicts.includes("model-folder-unavailable") && (
              <div role="alert" className="space-y-2 text-warning">
                <p>{tr("folderConflict")}</p>
                <Button
                  variant="secondary"
                  disabled={busy || restarting}
                  onClick={() =>
                    void run(async () => {
                      const result = await settingsTransfer.resolveFolder(
                        preview.id,
                      );
                      if (result) setPreview(result);
                    })
                  }
                >
                  {tr("chooseFolder")}
                </Button>
              </div>
            )}
            {preview.conflicts.includes("audio-device-unavailable") && (
              <div role="alert" className="space-y-2 text-warning">
                <p>{tr("audioConflict")}</p>
                <Button
                  variant="secondary"
                  disabled={busy || restarting}
                  onClick={() =>
                    void run(async () => {
                      setPreview(
                        await settingsTransfer.resolveAudio(preview.id),
                      );
                    })
                  }
                >
                  {tr("defaultAudio")}
                </Button>
              </div>
            )}
            <div className="flex flex-wrap gap-2">
              <Button
                variant="secondary"
                disabled={busy || restarting}
                onClick={() =>
                  void run(async () => {
                    await settingsTransfer.cancel(preview.id);
                    setPreview(null);
                    setNotice(tr("cancelled"));
                  })
                }
              >
                {tr("cancel")}
              </Button>
              <Button
                disabled={busy || restarting || preview.conflicts.length > 0}
                onClick={() =>
                  void run(async () => {
                    const result = await settingsTransfer.apply(preview.id);
                    if (result.status === "restarting") {
                      setRestarting(true);
                      setNotice(tr("restarting"));
                    }
                  })
                }
              >
                {tr("confirm")}
              </Button>
            </div>
          </section>
        )}
      </div>
    </SettingsGroup>
  );
}
