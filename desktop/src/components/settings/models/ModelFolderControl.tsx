import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { commands, type ModelRootCandidate } from "@/bindings";
import { useModelStore, modelRootError } from "@/stores/modelStore";

/** Shared first-run and Settings folder flow. Validation never applies a root. */
export function ModelFolderControl({ preview = false }: { preview?: boolean }) {
  const { t } = useTranslation();
  const {
    root,
    loadModels,
    generation,
    rootApplied,
    rescanLocalModels,
    isRescanning,
    scanProgress,
  } = useModelStore();
  const [candidate, setCandidate] = useState<ModelRootCandidate | null>(null);
  const [pending, setPending] = useState<
    "choose" | "validate" | "apply" | null
  >(null);
  const [error, setError] = useState<string | null>(null);
  const active = useRef<string | null>(null);
  const mounted = useRef(true);
  const locationRequest = useRef(0);
  const revision = root?.revision;

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      const id = active.current;
      active.current = null;
      if (id) {
        useModelStore.getState().finishRootValidation(id);
        void commands.cancelModelRootScan(id).catch(() => {});
      }
    };
  }, []);
  useEffect(() => {
    const id = active.current;
    active.current = null;
    if (id) {
      useModelStore.getState().finishRootValidation(id);
      void commands.cancelModelRootScan(id).catch(() => {});
    }
    setCandidate(null);
    setPending(null);
    setError(null);
  }, [generation, revision]);

  const owns = (id: string, expected: number) =>
    mounted.current &&
    active.current === id &&
    useModelStore.getState().generation === expected;
  async function choose() {
    if (preview) return;
    const previous = active.current;
    if (previous) {
      useModelStore.getState().finishRootValidation(previous);
      void commands.cancelModelRootScan(previous).catch(() => {});
    }
    const id = crypto.randomUUID();
    const expected = generation;
    active.current = id;
    setCandidate(null);
    setError(null);
    setPending("choose");
    try {
      const folder = await open({
        directory: true,
        multiple: false,
        title: t("notype.modelFolder.choose"),
      });
      if (!owns(id, expected) || typeof folder !== "string") return;
      setPending("validate");
      useModelStore.getState().beginRootValidation(id);
      const result = await commands.validateModelRoot(folder, id);
      if (!owns(id, expected)) return;
      if (
        result.status === "ok" &&
        result.data.request_id === id &&
        result.data.root.generation === expected &&
        result.data.root.revision === revision
      )
        setCandidate(result.data);
      else
        setError(
          result.status === "error" ? modelRootError(result.error) : "stale",
        );
    } catch {
      if (owns(id, expected)) setError("unavailable");
    } finally {
      if (owns(id, expected)) {
        useModelStore.getState().finishRootValidation(id);
        setPending(null);
      }
    }
  }
  async function cancel() {
    if (preview) return;
    const id = active.current;
    active.current = null;
    setCandidate(null);
    setPending(null);
    setError(null);
    if (id) {
      useModelStore.getState().finishRootValidation(id);
      try {
        await commands.cancelModelRootScan(id);
      } catch {
        /* cancellation is best effort; ownership already revoked */
      }
    }
  }
  async function apply() {
    const id = active.current;
    if (preview || !candidate || !id || revision === undefined) return;
    const expected = generation;
    setPending("apply");
    setError(null);
    try {
      const result = await commands.applyModelRoot(
        candidate.candidate_id,
        candidate.root.revision,
      );
      if (!owns(id, expected)) return;
      if (result.status === "error") {
        setError(modelRootError(result.error));
        return;
      }
      if (result.data.generation < expected) {
        setError("stale");
        return;
      }
      active.current = null;
      setCandidate(null);
      setPending(null);
      await rootApplied(result.data);
    } catch {
      if (owns(id, expected)) setError("unavailable");
    } finally {
      if (owns(id, expected)) setPending(null);
    }
  }
  async function showLocation() {
    if (preview || !root) return;
    const expected = generation;
    const request = ++locationRequest.current;
    const ownsLocation = () =>
      mounted.current &&
      locationRequest.current === request &&
      useModelStore.getState().generation === expected;
    setError(null);
    try {
      const result = await commands.showModelRoot(expected);
      if (ownsLocation() && result.status === "error")
        setError(modelRootError(result.error));
    } catch {
      if (ownsLocation()) setError("unavailable");
    }
  }
  const status = (view: typeof root) =>
    view && (
      <>
        <p className="break-all text-sm">{view.folder}</p>
        <p className="text-sm" role="status">
          {t(`notype.modelFolder.states.${view.status}`)}
        </p>
        <p className="text-sm">
          {t("notype.modelFolder.modelCount", { count: view.model_count })}
        </p>
        {!view.writable && (
          <p className="text-sm">{t("notype.modelFolder.readonly")}</p>
        )}
        {view.busy && <p className="text-sm">{t("notype.modelFolder.busy")}</p>}
      </>
    );
  return (
    <section
      aria-label={t("notype.modelFolder.title")}
      className="rounded-lg border border-mid-gray/30 p-4 space-y-3 text-left"
    >
      <h2 className="font-medium">{t("notype.modelFolder.title")}</h2>
      <p className="text-sm text-text/70">
        {t("notype.modelFolder.description")}
      </p>
      {status(root)}
      {!root && (
        <button
          type="button"
          disabled={preview}
          onClick={() => void loadModels()}
          className="px-3 py-2 rounded border border-mid-gray/40 disabled:opacity-50"
        >
          {t("notype.modelFolder.reload")}
        </button>
      )}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          disabled={preview || !root || pending === "apply"}
          onClick={() => void choose()}
          className="px-3 py-2 rounded border border-mid-gray/40 disabled:opacity-50"
        >
          {t("notype.modelFolder.choose")}
        </button>
        <button
          type="button"
          disabled={preview || isRescanning || !root}
          onClick={() => void rescanLocalModels()}
          className="px-3 py-2 rounded border border-mid-gray/40 disabled:opacity-50"
        >
          {t("notype.modelFolder.rescan")}
        </button>
        <button
          type="button"
          disabled={preview || !root}
          onClick={() => void showLocation()}
          className="px-3 py-2 rounded border border-mid-gray/40 disabled:opacity-50"
        >
          {t("notype.modelFolder.showLocation")}
        </button>
      </div>
      {pending === "validate" && (
        <p role="status">{t("notype.modelFolder.validating")}</p>
      )}
      {scanProgress && (
        <p role="status">
          {t("notype.modelFolder.progress", {
            checked: scanProgress.checked,
            total: scanProgress.total,
          })}
        </p>
      )}
      {candidate && (
        <div className="space-y-2 border-t border-mid-gray/30 pt-3">
          <h3 className="font-medium">{t("notype.modelFolder.candidate")}</h3>
          {status(candidate.root)}
          <button
            type="button"
            disabled={
              pending === "apply" ||
              !["ready", "no_models"].includes(candidate.root.status)
            }
            onClick={() => void apply()}
            className="px-3 py-2 rounded bg-logo-primary text-white disabled:opacity-50"
          >
            {t("notype.modelFolder.apply")}
          </button>
        </div>
      )}
      {(candidate || pending === "validate") && (
        <button
          type="button"
          disabled={pending === "apply"}
          onClick={() => void cancel()}
          className="px-3 py-2 rounded border border-mid-gray/40"
        >
          {t("notype.modelFolder.cancel")}
        </button>
      )}
      {error && (
        <p role="alert" className="text-sm text-warning">
          {t(`notype.modelFolder.errors.${error}`)}
        </p>
      )}
    </section>
  );
}
