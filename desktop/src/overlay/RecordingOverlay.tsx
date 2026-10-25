import { reportCaptureObservation } from "./contracts";
import { listen } from "@tauri-apps/api/event";
import React, { useEffect, useLayoutEffect, useRef, useState } from "react";
import "./RecordingOverlay.css";
import { commands, events } from "@/bindings";
import type {
  StreamPhase,
  StreamPhaseEvent,
  StreamTextEvent,
  StreamWorkKind,
} from "@/bindings";
import i18n, { getSupportedLanguage } from "@/i18n";
import { getLanguageDirection } from "@/lib/utils/rtl";

import type {
  OverlayState,
  OverlayIdentity as Identity,
  ShowOverlayEvent as ShowEvent,
  RecordingReadyEvent as ReadyEvent,
  HideOverlayEvent as HideEvent,
} from "./contracts";

// Number of reactive bars in the waveform (the simple, smoothed style shared by
// every overlay form). Mic levels arrive as 16 FFT buckets; we take the first N.
const WAVE_BARS = 9;

type Capture = Identity & {
  captureRevision: string;
  closed: boolean;
  hidden: boolean;
  origin: number | null;
  commitObserved: boolean;
};
const validId = (value: unknown): value is string =>
  typeof value === "string" && /^[1-9][0-9]{0,19}$/.test(value);
const newer = (left: string, right: string) => BigInt(left) > BigInt(right);

const RecordingOverlay: React.FC = () => {
  const [language, setLanguage] = useState(i18n.language);
  const t = i18n.getFixedT(language);
  const capture = useRef<Capture | null>(null);
  const pendingReady = useRef<ReadyEvent | null>(null);
  const [captureId, setCaptureId] = useState("");
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>("recording");
  // `Stream::play()` returning does not mean hardware callbacks are flowing.
  // Stay visually in an arming state until the backend processes the first
  // actual microphone sample chunk.
  const [captureReady, setCaptureReady] = useState(false);
  const [levels, setLevels] = useState<number[]>(Array(WAVE_BARS).fill(0));
  const [streamText, setStreamText] = useState<StreamTextEvent>({
    capture_id: "",
    committed: "",
    tentative: "",
  });
  const [phase, setPhase] = useState<StreamPhase>("listening");
  const [workKind, setWorkKind] = useState<StreamWorkKind>("transcribing");
  const [elapsed, setElapsed] = useState(0);
  // Bumped on each new streaming session so the Live card remounts fresh (replays
  // the pop-in, and never animates in from the previous panel's open size).
  const [session, setSession] = useState(0);
  // Overlay placement (top vs bottom of the screen). The Live panel grows downward
  // from a top overlay (oldest line under the pill) and upward from a bottom one.
  const [position, setPosition] = useState<"top" | "bottom">("bottom");
  // True once live text overflows the cap. A top overlay fades its top edge only
  // while overflowing, so the resting first line stays crisp flush under the pill.
  const [overflowing, setOverflowing] = useState(false);

  const smoothedLevelsRef = useRef<number[]>(Array(16).fill(0));
  // Live-text scroll-back: the text region "sticks" to the newest line while the
  // user is at the bottom; if they scroll up to read history, auto-follow pauses
  // until they scroll back down.
  const capRef = useRef<HTMLDivElement>(null);
  const pinnedRef = useRef(true);
  const direction = getLanguageDirection(language);

  useEffect(() => {
    let disposed = false;
    const unlisteners = new Set<() => void>();
    const dispose = (unlisten: () => void) => {
      try {
        void Promise.resolve(unlisten()).catch(() => {});
      } catch {
        /* A failing transport must not prevent cleanup of its peers. */
      }
    };
    const register = (registration: Promise<() => void>) => {
      void registration
        .then((unlisten) => {
          if (disposed) dispose(unlisten);
          else unlisteners.add(unlisten);
        })
        .catch(() => {
          /* A rejected registration cannot become a leaked listener. */
        });
    };
    const stopClock = () => {
      const current = capture.current;
      if (current) {
        current.closed = true;
        current.origin = null;
      }
      pendingReady.current = null;
      setCaptureReady(false);
      setElapsed(0);
    };
    const acceptReady = (payload: ReadyEvent) => {
      if (!validId(payload?.captureId) || !validId(payload?.captureRevision))
        return;
      const current = capture.current;
      if (!current || newer(payload.captureId, current.captureId)) {
        if (
          !pendingReady.current ||
          !newer(pendingReady.current.captureId, payload.captureId)
        )
          pendingReady.current = payload;
        return;
      }
      if (
        payload.captureId !== current.captureId ||
        payload.captureRevision !== current.captureRevision ||
        current.closed ||
        current.hidden ||
        current.origin !== null
      )
        return;
      current.origin = performance.now();
      void reportCaptureObservation(
        current.captureId,
        current.captureRevision,
        "received",
        0,
      ).catch(() => {});
      setElapsed(0);
      setCaptureReady(true);
    };
    register(
      listen<ShowEvent>("show-overlay", (event) => {
        if (disposed) return;
        const payload = event.payload;
        if (
          !validId(payload?.captureId) ||
          !validId(payload?.revision) ||
          !["recording", "streaming", "transcribing", "processing"].includes(
            payload.state,
          )
        )
          return;
        const previous = capture.current;
        if (
          previous &&
          (newer(previous.captureId, payload.captureId) ||
            (payload.captureId === previous.captureId &&
              (previous.hidden || newer(previous.revision, payload.revision))))
        )
          return;
        const capturing =
          payload.state === "recording" || payload.state === "streaming";
        const fresh = !previous || previous.captureId !== payload.captureId;
        if (!fresh && previous.closed && capturing) return;
        if (fresh) {
          capture.current = {
            captureId: payload.captureId,
            revision: payload.revision,
            captureRevision: payload.revision,
            closed: !capturing,
            hidden: false,
            origin: null,
            commitObserved: false,
          };
          setCaptureId(payload.captureId);
          setCaptureReady(false);
          setElapsed(0);
          smoothedLevelsRef.current = Array(16).fill(0);
          setLevels(Array(WAVE_BARS).fill(0));
          setStreamText({
            capture_id: payload.captureId,
            committed: "",
            tentative: "",
          });
          setPhase("listening");
          setWorkKind("transcribing");
          setSession((value) => value + 1);
        } else {
          capture.current!.revision = payload.revision;
        }
        if (!capturing) stopClock();
        setState(payload.state);
        setIsVisible(true);
        const early = pendingReady.current;
        if (early?.captureId === payload.captureId) {
          pendingReady.current = null;
          acceptReady(early);
        } else if (early && newer(payload.captureId, early.captureId))
          pendingReady.current = null;
        // Presentation becomes active before nonessential settings I/O. Its result
        // belongs to this exact request and can never resurrect a hidden capture.
        const settingsStarted = performance.now();
        void commands
          .getAppSettings()
          .then((settings) => {
            const current = capture.current;
            if (
              disposed ||
              !current ||
              current.hidden ||
              current.captureId !== payload.captureId ||
              current.revision !== payload.revision
            )
              return;
            void reportCaptureObservation(
              current.captureId,
              current.captureRevision,
              "settings",
              performance.now() - settingsStarted,
            ).catch(() => {});
            if (settings.status === "ok") {
              setPosition(
                settings.data.overlay_position === "top" ? "top" : "bottom",
              );
              setLanguage(
                getSupportedLanguage(settings.data.app_language) ?? "zh-TW",
              );
            }
          })
          .catch(() => {
            /* Preserve the last known presentation on settings failure. */
          });
      }),
    );
    register(
      listen<HideEvent>("hide-overlay", (event) => {
        if (disposed) return;
        const payload = event.payload;
        if (!validId(payload?.captureId) || !validId(payload?.revision)) return;
        const current = capture.current;
        if (
          current &&
          (newer(current.captureId, payload.captureId) ||
            (current.captureId === payload.captureId &&
              newer(current.revision, payload.revision)))
        )
          return;
        capture.current = {
          captureId: payload.captureId,
          revision: payload.revision,
          captureRevision: "",
          closed: true,
          hidden: true,
          origin: null,
          commitObserved: false,
        };
        stopClock();
        setIsVisible(false);
      }),
    );
    register(
      listen<ReadyEvent>("recording-ready", (event) => {
        if (!disposed) acceptReady(event.payload);
      }),
    );
    register(
      listen<number[]>("mic-level", (event) => {
        if (disposed || !capture.current || capture.current.closed) return;
        const smoothed = smoothedLevelsRef.current.map(
          (prev, index) => prev * 0.7 + (event.payload[index] || 0) * 0.3,
        );
        smoothedLevelsRef.current = smoothed;
        setLevels(smoothed.slice(0, WAVE_BARS));
      }),
    );
    register(
      events.streamTextEvent.listen((event) => {
        const current = capture.current;
        if (
          !disposed &&
          current &&
          !current.hidden &&
          event.payload.capture_id === current.captureId
        )
          setStreamText(event.payload);
      }),
    );
    register(
      events.streamPhaseEvent.listen((event) => {
        if (disposed) return;
        const payload: StreamPhaseEvent = event.payload;
        const current = capture.current;
        if (
          !current ||
          current.hidden ||
          payload.capture_id !== current.captureId ||
          !validId(payload.revision) ||
          newer(current.revision, payload.revision)
        )
          return;
        if (payload.phase !== "working" || current.hidden) return;
        current.revision = payload.revision;
        stopClock();
        setPhase(payload.phase);
        if (payload.kind) setWorkKind(payload.kind);
      }),
    );
    return () => {
      disposed = true;
      for (const unlisten of unlisteners) dispose(unlisten);
      unlisteners.clear();
      capture.current = null;
      pendingReady.current = null;
    };
  }, []);

  // Display elapsed starts at the first accepted ready event in this webview.
  // It is not an audio sample duration and uses no backend clock arithmetic.
  useEffect(() => {
    if (!isVisible || !captureReady || capture.current?.closed) return;
    const update = () => {
      const origin = capture.current?.origin;
      if (origin !== null && origin !== undefined)
        setElapsed(
          Math.max(0, Math.floor((performance.now() - origin) / 1000)),
        );
    };
    const id = setInterval(update, 250);
    return () => clearInterval(id);
  }, [state, phase, isVisible, captureReady, captureId]);

  // React commit observation is local to this webview, not paint or sample time.
  useLayoutEffect(() => {
    const current = capture.current;
    if (
      !current ||
      current.hidden ||
      current.closed ||
      !captureReady ||
      current.origin === null ||
      current.commitObserved
    )
      return;
    current.commitObserved = true;
    void reportCaptureObservation(
      current.captureId,
      current.captureRevision,
      "committed",
      performance.now() - current.origin,
    ).catch(() => {});
  }, [captureReady, captureId]);

  // Stick to the bottom as text streams in — but only while pinned, so a user who
  // has scrolled up to read history isn't yanked back down by the next chunk.
  useLayoutEffect(() => {
    const el = capRef.current;
    if (!el) return;
    // Fade the top edge only once text actually overflows the cap.
    setOverflowing(el.scrollHeight > el.clientHeight + 1);
    if (pinnedRef.current) el.scrollTop = el.scrollHeight;
  }, [streamText]);

  // Each fresh streaming session starts pinned to the bottom, fade cleared.
  useEffect(() => {
    pinnedRef.current = true;
    setOverflowing(false);
  }, [session]);

  if (!isVisible) return null;

  // Re-pin when the user is within ~a line of the bottom; unpin otherwise.
  const handleStreamScroll = () => {
    const el = capRef.current;
    if (!el) return;
    pinnedRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= 16;
  };

  const fmtTime = (s: number) =>
    `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;

  // ---- Shared building blocks (one visual language for every overlay form) ----
  const waveform = (
    <div className={`swave ${captureReady ? "ready" : "arming"}`}>
      {levels.map((v, i) => (
        <i
          key={i}
          style={{
            height: `${Math.max(3, Math.min(18, 3 + Math.pow(v, 0.7) * 15))}px`,
          }}
        />
      ))}
    </div>
  );

  const cancelBtn = (
    <button
      className="sx"
      aria-label="cancel"
      onClick={() => commands.cancelOperation()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path
          d="M4 4 L12 12 M12 4 L4 12"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        />
      </svg>
    </button>
  );

  // dot (left) | waveform (center) | timer + cancel (right) — same structure for
  // pill & panel, so the Live morph is a pure width change.
  const listeningRow = (showTimer: boolean, showCancel: boolean) => (
    <div className="sbase capture-row">
      <div className="sbase-l">
        <span className={`sdot ${captureReady ? "ready" : "arming"}`} />
      </div>
      {waveform}
      <div className="sbase-r">
        {showTimer && captureReady && (
          <span
            className="stimer"
            role="timer"
            aria-live="off"
            aria-label={t("overlay.elapsed")}
          >
            {fmtTime(elapsed)}
          </span>
        )}
        {showCancel && cancelBtn}
      </div>
    </div>
  );

  // spinner (left) | label (center) | cancel (right) — same 3-zone grid as the
  // listening row, so the label is centered.
  const workingRow = (label: string, showCancel: boolean) => (
    <div className="sbase">
      <div className="sbase-l">
        <span className="sspinner" />
      </div>
      <span className="swork-label">{label}</span>
      <div className="sbase-r">{showCancel && cancelBtn}</div>
    </div>
  );

  // ---- Live overlay: a pill that sculpts open into a panel ----
  if (state === "streaming") {
    const hasText =
      streamText.committed.length > 0 || streamText.tentative.length > 0;
    const working = phase === "working";
    // Keep the panel open whenever there's text — even while finalizing — so the
    // transcript stays put under a working spinner instead of collapsing and
    // squishing the text mid-stream. Only fall back to the small working pill
    // when there was no text to preserve.
    const open = hasText;
    const collapsed = working && !hasText;

    return (
      <div dir={direction} className={`ov-stage ${position}`}>
        <div
          key={session}
          className={`scard ${open ? "open" : ""} ${collapsed ? "working" : ""} ${
            isVisible ? "" : "leaving"
          }`}
        >
          <div className="stext">
            <div className="stext-clip">
              <div
                className={`stext-cap ${overflowing ? "overflowing" : ""}`}
                ref={capRef}
                onScroll={handleStreamScroll}
              >
                <p>
                  <span className="committed">
                    {streamText.committed ? streamText.committed + " " : ""}
                  </span>
                  <span className="tentative">{streamText.tentative}</span>
                  {/* Drop the blinking caret once finalizing — it's no longer
                      capturing, and a static spinner conveys the work. */}
                  {!working && <span className="scaret" />}
                </p>
              </div>
            </div>
          </div>
          {working
            ? workingRow(
                workKind === "polishing"
                  ? t("overlay.processing")
                  : t("overlay.transcribing"),
                true,
              )
            : listeningRow(true, true)}
        </div>
      </div>
    );
  }

  // ---- Minimal overlay: exactly one row at a time — waveform (recording), or a
  // spinner + label (transcribing / processing). Never both. The pill animates its
  // width between them; the cancel button is in both rows so it stays put.
  const working = state === "transcribing" || state === "processing";
  const workLabel =
    state === "processing"
      ? t("overlay.processing")
      : t("overlay.transcribing");

  return (
    <div
      dir={direction}
      className={`ov-stage ${position} ov-fade ${isVisible ? "show" : ""}`}
    >
      <div
        className={`scard compact ${working && isVisible ? "cworking" : ""}`}
      >
        {working ? workingRow(workLabel, true) : listeningRow(true, true)}
      </div>
    </div>
  );
};

export default RecordingOverlay;
