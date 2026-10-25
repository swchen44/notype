import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

const defaults = {
  transcribe: "alt_right",
  translation: "alt_right+shift_right",
  ask: "alt_right+space",
  stop: "alt_right",
};
type BindingId = keyof typeof defaults;

function BindingEditor({ id }: { id: BindingId }) {
  const { t } = useTranslation();
  const { settings, updateBinding, resetBinding } = useSettings();
  const stored = settings?.bindings?.[id]?.current_binding ?? defaults[id];
  const [value, setValue] = useState(stored);
  const [status, setStatus] = useState("");
  const [working, setWorking] = useState(false);
  useEffect(() => setValue(stored), [stored]);
  async function save(reset: boolean) {
    setWorking(true);
    try {
      if (reset) await resetBinding(id);
      else await updateBinding(id, value.trim());
      setStatus(t("notype.hotkeys.saved"));
    } catch {
      setStatus(t("notype.hotkeys.invalid"));
    } finally {
      setWorking(false);
    }
  }
  return (
    <SettingContainer title={t(`notype.hotkeys.${id}`)} description="" grouped>
      <div className="space-y-2">
        <div className="flex flex-wrap gap-2">
          <Input
            aria-label={t(`notype.hotkeys.${id}`)}
            value={value}
            disabled={working}
            onChange={(event) => {
              setValue(event.target.value);
              setStatus("");
            }}
          />
          <Button
            size="sm"
            disabled={working || value === stored}
            onClick={() => void save(false)}
          >
            {t("notype.hotkeys.save")}
          </Button>
          <Button
            size="sm"
            variant="secondary"
            disabled={working}
            onClick={() => void save(true)}
          >
            {t("notype.hotkeys.reset")}
          </Button>
        </div>
        {status && (
          <p role="status" className="text-sm">
            {status}
          </p>
        )}
      </div>
    </SettingContainer>
  );
}

export function ScenarioShortcuts() {
  const { t } = useTranslation();
  return (
    <>
      <p className="px-4 py-2 text-sm text-text/70">
        {t("notype.hotkeys.description")}
      </p>
      {(Object.keys(defaults) as BindingId[]).map((id) => (
        <BindingEditor key={id} id={id} />
      ))}
      <p className="px-4 py-2 text-sm text-text/70">
        {t("notype.hotkeys.cancel")}
      </p>
    </>
  );
}
