import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import { produce } from "immer";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type ModelInfo,
  type ModelArtifactView,
  type ModelRootView,
  type ModelRootScanProgress,
  type ModelGenerationEvent,
} from "@/bindings";

interface DownloadProgress {
  generation: number;
  model_id: string;
  downloaded: number;
  total: number;
  percentage: number;
}
interface DownloadStats {
  startTime: number;
  lastUpdate: number;
  totalDownloaded: number;
  speed: number;
}

/** Native errors are finite codes; raw paths and transport rejections stay out of UI. */
export function modelRootError(error: unknown): string {
  const codes = [
    "busy",
    "invalid",
    "stale",
    "cancelled",
    "unavailable",
    "persistence-failed",
    "readonly",
    "limit",
  ];
  return typeof error === "string" &&
    codes.some((code) => error === `model-root-${code}`)
    ? error.slice("model-root-".length)
    : "unavailable";
}

interface ModelsStore {
  models: ModelInfo[];
  currentModel: string;
  generation: number;
  root: ModelRootView | null;
  artifacts: Record<string, ModelArtifactView>;
  downloadingModels: Record<string, true>;
  verifyingModels: Record<string, true>;
  extractingModels: Record<string, true>;
  downloadProgress: Record<string, DownloadProgress>;
  downloadStats: Record<string, DownloadStats>;
  scanProgress: ModelRootScanProgress | null;
  beginRootValidation: (requestId: string) => void;
  finishRootValidation: (requestId: string) => void;
  loading: boolean;
  error: string | null;
  initialized: boolean;
  isRescanning: boolean;
  initialize: () => Promise<void>;
  loadModels: () => Promise<void>;
  loadCurrentModel: () => Promise<void>;
  rootApplied: (root: ModelRootView) => Promise<void>;
  rescanLocalModels: () => Promise<void>;
  selectModel: (modelId: string) => Promise<boolean>;
  downloadModel: (modelId: string) => Promise<boolean>;
  cancelDownload: (modelId: string) => Promise<boolean>;
  deleteModel: (modelId: string) => Promise<boolean>;
  getModelInfo: (modelId: string) => ModelInfo | undefined;
  isModelDownloading: (modelId: string) => boolean;
  isModelVerifying: (modelId: string) => boolean;
  isModelExtracting: (modelId: string) => boolean;
  getDownloadProgress: (modelId: string) => DownloadProgress | undefined;
  setModels: (models: ModelInfo[]) => void;
  setCurrentModel: (modelId: string) => void;
  setError: (error: string | null) => void;
  setLoading: (loading: boolean) => void;
}

export const useModelStore = create<ModelsStore>()(
  subscribeWithSelector((set, get) => {
    let catalogRequest = 0;
    let rescanRequest = 0;
    let selectionRequest = 0;
    let validationRequest: string | null = null;
    const operations = new Map<string, symbol>();
    let initializing: Promise<void> | null = null;
    const transient = () => ({
      downloadingModels: {},
      verifyingModels: {},
      extractingModels: {},
      downloadProgress: {},
      downloadStats: {},
      scanProgress: null,
      isRescanning: false,
    });
    const advance = (generation: number) => {
      if (generation <= get().generation) return;
      ++catalogRequest;
      ++rescanRequest;
      ++selectionRequest;
      validationRequest = null;
      operations.clear();
      set({ generation, models: [], artifacts: {}, ...transient() });
    };
    const clearDownload = (modelId: string) =>
      set(
        produce((state: ModelsStore) => {
          delete state.downloadingModels[modelId];
          delete state.verifyingModels[modelId];
          delete state.extractingModels[modelId];
          delete state.downloadProgress[modelId];
          delete state.downloadStats[modelId];
        }),
      );
    const ownsEvent = (event: ModelGenerationEvent) =>
      event.generation === get().generation;
    const operation = async (
      kind: "download" | "cancel" | "delete",
      modelId: string,
    ): Promise<boolean> => {
      const generation = get().generation;
      const token = Symbol();
      const key = `${kind}:${modelId}`;
      operations.set(key, token);
      const owns = () =>
        get().generation === generation && operations.get(key) === token;
      if (
        kind !== "cancel" &&
        (!get().root?.writable ||
          !get().artifacts[modelId]?.[
            kind === "delete" ? "can_delete" : "can_download"
          ])
      ) {
        if (owns()) set({ error: "readonly" });
        operations.delete(key);
        return false;
      }
      set({ error: null });
      if (kind === "download")
        set(
          produce((state: ModelsStore) => {
            state.downloadingModels[modelId] = true;
            state.downloadProgress[modelId] = {
              generation,
              model_id: modelId,
              downloaded: 0,
              total: 0,
              percentage: 0,
            };
          }),
        );
      try {
        const result = await (kind === "download"
          ? commands.downloadModel(modelId, generation)
          : kind === "cancel"
            ? commands.cancelDownload(modelId, generation)
            : commands.deleteModel(modelId, generation));
        if (!owns()) return false;
        if (result.status === "error") {
          set({ error: modelRootError(result.error) });
          if (kind === "download") clearDownload(modelId);
          return false;
        }
        // Cancellation acknowledgement is not worker completion. Terminal events
        // or a coherent catalog, rather than this Promise, clear its progress.
        await get().loadModels();
        return owns();
      } catch {
        if (owns()) {
          set({ error: "unavailable" });
          if (kind === "download") clearDownload(modelId);
        }
        return false;
      } finally {
        if (owns()) operations.delete(key);
      }
    };
    return {
      models: [],
      currentModel: "",
      generation: 0,
      root: null,
      artifacts: {},
      ...transient(),
      loading: true,
      error: null,
      initialized: false,
      setModels: (models) => set({ models }),
      setCurrentModel: (currentModel) => set({ currentModel }),
      setError: (error) => set({ error }),
      setLoading: (loading) => set({ loading }),
      beginRootValidation: (requestId) => {
        validationRequest = requestId;
        set({ scanProgress: null });
      },
      finishRootValidation: (requestId) => {
        if (validationRequest === requestId) {
          validationRequest = null;
          set({ scanProgress: null });
        }
      },
      loadModels: async () => {
        const request = ++catalogRequest;
        const generation = get().generation;
        const owns = () =>
          request === catalogRequest && generation === get().generation;
        try {
          const result = await commands.getModelCatalog();
          if (!owns()) return;
          if (result.status === "error") {
            set({ error: modelRootError(result.error) });
            return;
          }
          const catalog = result.data;
          if (
            catalog.generation < generation ||
            catalog.root.generation !== catalog.generation
          )
            return;
          if (catalog.generation > generation) advance(catalog.generation);
          const downloads: Record<string, true> = {};
          catalog.models
            .filter((model) => model.is_downloading)
            .forEach((model) => {
              downloads[model.id] = true;
            });
          set({
            generation: catalog.generation,
            root: catalog.root,
            models: catalog.models,
            currentModel: catalog.current_model,
            artifacts: catalog.artifacts,
            downloadingModels: downloads,
            error: null,
            loading: false,
          });
        } catch {
          if (owns()) set({ error: "unavailable" });
        } finally {
          if (owns()) set({ loading: false });
        }
      },
      // One catalog response owns the root, preference and artifact projection.
      loadCurrentModel: async () => get().loadModels(),
      rootApplied: async (root) => {
        if (root.generation < get().generation) return;
        advance(root.generation);
        set({ root });
        await get().loadModels();
      },
      rescanLocalModels: async () => {
        const request = ++rescanRequest;
        const generation = get().generation;
        const owns = () =>
          request === rescanRequest && generation === get().generation;
        set({ isRescanning: true, error: null });
        try {
          const result = await commands.rescanLocalModels();
          if (!owns()) return;
          if (result.status === "error")
            set({ error: modelRootError(result.error) });
          else if (result.data.generation === generation)
            await get().loadModels();
        } catch {
          if (owns()) set({ error: "unavailable" });
        } finally {
          if (owns()) set({ isRescanning: false });
        }
      },
      selectModel: async (modelId) => {
        const request = ++selectionRequest;
        const generation = get().generation;
        const owns = () =>
          request === selectionRequest && generation === get().generation;
        if (
          !get().models.some(
            (model) => model.id === modelId && model.is_downloaded,
          ) ||
          !["verified", "custom_unverified"].includes(
            get().artifacts[modelId]?.status ?? "unavailable",
          )
        )
          return false;
        set({ error: null });
        try {
          const result = await commands.setActiveModel(modelId, generation);
          if (!owns()) return false;
          if (result.status === "error") {
            set({ error: modelRootError(result.error) });
            return false;
          }
          await get().loadModels();
          return owns() && get().currentModel === modelId;
        } catch {
          if (owns()) set({ error: "unavailable" });
          return false;
        }
      },
      downloadModel: (modelId) => operation("download", modelId),
      cancelDownload: (modelId) => operation("cancel", modelId),
      deleteModel: (modelId) => operation("delete", modelId),
      getModelInfo: (modelId) =>
        get().models.find((model) => model.id === modelId),
      isModelDownloading: (modelId) => modelId in get().downloadingModels,
      isModelVerifying: (modelId) => modelId in get().verifyingModels,
      isModelExtracting: (modelId) => modelId in get().extractingModels,
      getDownloadProgress: (modelId) => get().downloadProgress[modelId],
      initialize: async () => {
        if (initializing) return initializing;
        if (get().initialized) return;
        initializing = (async () => {
          const on = <T>(name: string, handler: (payload: T) => void) =>
            listen<T>(name, (event) => handler(event.payload));
          const refresh = (event: ModelGenerationEvent) => {
            if (event.generation < get().generation) return;
            advance(event.generation);
            void get().loadModels();
          };
          await Promise.all([
            on<ModelGenerationEvent>("model-root-changed", refresh),
            on<ModelGenerationEvent>("models-updated", refresh),
            on<ModelGenerationEvent>("model-state-changed", refresh),
            on<ModelGenerationEvent>("model-deleted", refresh),
            on<ModelRootScanProgress>(
              "model-root-scan-progress",
              (progress) => {
                if (
                  progress.generation === get().generation &&
                  progress.request_id === validationRequest
                )
                  set({ scanProgress: progress });
              },
            ),
            on<DownloadProgress>("model-download-progress", (progress) => {
              if (!ownsEvent(progress)) return;
              const now = Date.now();
              set(
                produce((state: ModelsStore) => {
                  state.downloadProgress[progress.model_id] = progress;
                  state.downloadingModels[progress.model_id] = true;
                  const current = state.downloadStats[progress.model_id];
                  const elapsed = current
                    ? (now - current.lastUpdate) / 1000
                    : 0;
                  const speed =
                    current && elapsed > 0.5
                      ? Math.max(
                          0,
                          (progress.downloaded - current.totalDownloaded) /
                            1048576 /
                            elapsed,
                        )
                      : (current?.speed ?? 0);
                  state.downloadStats[progress.model_id] = {
                    startTime: current?.startTime ?? now,
                    lastUpdate: now,
                    totalDownloaded: progress.downloaded,
                    speed,
                  };
                }),
              );
            }),
            ...[
              "model-download-complete",
              "model-download-cancelled",
              "model-download-failed",
              "model-extraction-failed",
            ].map((name) =>
              on<ModelGenerationEvent>(name, (event) => {
                if (!ownsEvent(event) || !event.model_id) return;
                clearDownload(event.model_id);
                if (event.error) set({ error: modelRootError(event.error) });
                else void get().loadModels();
              }),
            ),
            ...(
              [
                ["model-verification-started", "verifyingModels", true],
                ["model-verification-completed", "verifyingModels", false],
                ["model-extraction-started", "extractingModels", true],
                ["model-extraction-completed", "extractingModels", false],
              ] as const
            ).map(([name, field, started]) =>
              on<ModelGenerationEvent>(name, (event) => {
                if (!ownsEvent(event) || !event.model_id) return;
                const id = event.model_id;
                set(
                  produce((state: ModelsStore) => {
                    if (started) state[field][id] = true;
                    else delete state[field][id];
                  }),
                );
                if (!started) void get().loadModels();
              }),
            ),
          ]);
          set({ initialized: true });
          await get().loadModels();
        })();
        try {
          await initializing;
        } finally {
          initializing = null;
        }
      },
    };
  }),
);
