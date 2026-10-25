import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ConnectionDraft, ConnectionView, KeyAction } from "@/bindings";
import { connectionDraft, endpointPreview } from "@/lib/providerForm";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";

export function ConnectionEditor({
  connection,
  busy,
  onSave,
}: {
  connection: ConnectionView;
  busy: boolean;
  onSave: (draft: ConnectionDraft, keyAction: KeyAction) => Promise<boolean>;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(() => connectionDraft(connection));
  return (
    <section
      aria-label={t("notype.connections.editor")}
      className="space-y-4 border-t border-mid-gray/30 pt-4"
    >
      <h3 className="font-semibold">{t("notype.connections.editor")}</h3>
      <fieldset disabled={busy} className="space-y-3 min-w-0">
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.connections.name")}
          <Input
            aria-label={t("notype.connections.name")}
            value={draft.display_name}
            onChange={(event) =>
              setDraft({ ...draft, display_name: event.target.value })
            }
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          {t("notype.text.model")}
          <Input
            aria-label={t("notype.text.model")}
            value={draft.model}
            autoComplete="off"
            spellCheck={false}
            onChange={(event) =>
              setDraft({ ...draft, model: event.target.value })
            }
          />
        </label>
        <p className="text-sm text-text/70">
          {t("notype.connections.endpointPreview")}:{" "}
          {endpointPreview(draft) ?? t("notype.connections.invalidEndpoint")}
        </p>
        <Button
          disabled={!draft.display_name.trim()}
          onClick={() => void onSave(draft, { action: "keep" })}
        >
          {t("notype.connections.save")}
        </Button>
      </fieldset>
    </section>
  );
}
