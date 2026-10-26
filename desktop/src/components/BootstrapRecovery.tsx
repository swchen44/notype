import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type BootstrapStatus } from "@/bindings";

export function useBootstrapRecovery() {
  const [status, setStatus] = useState<BootstrapStatus | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    commands
      .getBootstrapStatus()
      .then((value) => {
        if (active && value) setStatus(value);
      })
      .catch(() => {
        if (active)
          setStatus({ state: "incomplete", code: null, can_retry: true });
      });
    return () => {
      active = false;
    };
  }, []);
  const retry = useCallback(async () => {
    setBusy(true);
    try {
      const result = await commands.retryOfflineBootstrap();
      if (result.status === "ok") setStatus(result.data);
    } finally {
      setBusy(false);
    }
  }, []);
  return { status, busy, retry };
}

export function BootstrapRecovery({
  status,
  busy,
  retry,
}: {
  status: BootstrapStatus | null;
  busy: boolean;
  retry: () => Promise<void>;
}) {
  const { t } = useTranslation();
  if (!status || status.state === "ready") return null;
  return (
    <section role="status" className="w-full border rounded-lg p-4 space-y-2">
      <h2 className="font-medium">{t("notype.bootstrap.title")}</h2>
      <p>{t("notype.bootstrap.description")}</p>
      <button
        type="button"
        disabled={busy || !status.can_retry}
        onClick={() => {
          void retry();
        }}
        className="border rounded px-3 py-1 disabled:opacity-50"
      >
        {t(busy ? "notype.bootstrap.retrying" : "notype.bootstrap.retry")}
      </button>
    </section>
  );
}
