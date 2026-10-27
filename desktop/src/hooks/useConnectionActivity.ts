import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type ConnectionActivityView,
  type ConnectionSettingsView,
} from "@/bindings";

export function connectionConfigurationChanged() {
  window.dispatchEvent(new Event("notype-text-configuration-changed"));
}
export function useConnectionActivity() {
  const [configuration, setConfiguration] =
    useState<ConnectionSettingsView | null>(null);
  const [activity, setActivity] = useState<ConnectionActivityView[]>([]);
  const [failed, setFailed] = useState(false);
  const epoch = useRef(0);
  const refresh = useCallback(async () => {
    const current = ++epoch.current;
    try {
      const [settings, usage] = await Promise.all([
        commands.getTextConfiguration(),
        commands.getConnectionActivity(),
      ]);
      if (current !== epoch.current) return;
      if (settings.status !== "ok" || usage.status !== "ok")
        throw new Error("activity-unavailable");
      if (
        usage.data.some(
          (row) => row.configuration_revision !== settings.data.revision,
        )
      )
        throw new Error("activity-stale");
      setConfiguration(settings.data);
      setActivity(usage.data);
      setFailed(false);
    } catch {
      if (current === epoch.current) setFailed(true);
    }
  }, []);
  useEffect(() => {
    void refresh();
    const update = () => void refresh();
    window.addEventListener("notype-text-configuration-changed", update);
    window.addEventListener("focus", update);
    const stops = [
      "text-connection-activity-changed",
      "session-history-changed",
    ].map((name) => listen(name, update));
    return () => {
      epoch.current++;
      window.removeEventListener("notype-text-configuration-changed", update);
      window.removeEventListener("focus", update);
      for (const stop of stops)
        void stop.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [refresh]);
  return { configuration, activity, failed, refresh };
}
