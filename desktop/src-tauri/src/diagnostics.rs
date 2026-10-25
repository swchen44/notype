//! Privacy boundary for support reports. Only typed, bounded metadata enters
//! this module. It never reads raw logs, settings documents, history or audio.

use crate::capture_startup::{CaptureEvent, CaptureOutcome, CaptureSpan};
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[path = "diagnostic_privacy.rs"]
mod privacy;
#[path = "diagnostic_service.rs"]
pub mod service;

/// Maximum app-run stage records retained; older records are discarded first.
pub const MAX_EVENTS: usize = 128;
const MAX_DURATION_MS: u64 = 86_400_000;
static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

macro_rules! labels {
    ($doc:literal, $name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug)]
        pub enum $name {
            $(#[doc = concat!("Allowlisted code `", $label, "`; contains no caller-provided text.")]
            $variant),+
        }
        impl $name {
            /// Stable report code. Never formats caller-provided data.
            pub fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }
        }
    };
}

labels!("Allowlisted product stages. Callers record stage boundaries, never session IDs, keyboard text, model names or content.", Phase {
    Startup => "startup",
    CaptureTiming => "capture-timing",
    ShortcutAccepted => "shortcut-accepted",
    ShortcutDispatch => "shortcut-dispatch",
    RecordingStart => "recording-start",
    AudioStreamOpen => "audio-stream-open",
    MicrophoneReady => "microphone-ready",
    RecordingStopped => "recording-stopped",
    ModelLoad => "model-load",
    Transcription => "transcription",
    HistorySave => "history-save",
    TextProcessing => "text-processing",
    Delivery => "delivery",
});
labels!("Allowlisted stage outcomes. Started without a later completion can indicate waiting or interruption; it does not identify a cause.", Outcome {
    Started => "started", Ok => "ok", Failed => "failed",
    Cancelled => "cancelled", Unavailable => "unavailable",
});
labels!("Coarse OS family only; excludes hostnames, account names and machine identifiers.", Platform { Windows => "windows", Macos => "macos", Linux => "linux", Other => "other" });
labels!("Local raw-log verbosity metadata. It never changes the report's privacy allowlist.", LogLevel { Error => "error", Warn => "warn", Info => "info", Debug => "debug", Trace => "trace" });
labels!("Active local-log metadata classification, obtained without reading log contents or exporting its path. Empty does not imply no errors.", LogFileStatus { Empty => "empty", Present => "present", Missing => "missing", Unavailable => "unavailable" });

/// All fields except version are enums or booleans. Version is restricted to a
/// short numeric release identifier at the report boundary.
pub struct ReportContext<'a> {
    /// Borrowed release version; report creation accepts at most 32 digits/dots.
    pub version: &'a str,
    /// Coarse OS family, without identifying host metadata.
    pub platform: Platform,
    /// Configured local raw-log level, independent of report collection.
    pub log_level: LogLevel,
    /// Whether local developer mode is enabled; reports work when disabled.
    pub debug_mode: bool,
    /// Configured microphone mode, without names or device identifiers.
    pub always_on_microphone: bool,
    /// Whether a model preference exists; the preference string is excluded.
    pub model_selected: bool,
    /// Catalog download availability; this is not a successful-load guarantee.
    pub model_available: bool,
    /// Whether selected model metadata is custom; excludes its name and path.
    pub model_is_custom: bool,
    /// Active sink metadata only; no log contents cross this boundary.
    pub local_log: LogFileStatus,
}

/// Immutable owned preview/export snapshot. Its opaque ID identifies this
/// snapshot, not a person or session. Core clones retain identical ID/text;
/// the native adapter separately keeps only the latest preview until restart.
#[derive(Clone)]
pub struct Report {
    id: String,
    text: String,
}

impl Report {
    /// Returns this snapshot's report ID for preview matching and a safe issue
    /// summary. It is not an authentication token or a persistent user ID.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// Returns the exact allowlisted text exported by this owned snapshot.
    /// Later events do not change it, and callers cannot edit it in place.
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Clone)]
struct Event {
    elapsed_ms: u64,
    phase: Phase,
    outcome: Outcome,
    duration_ms: Option<u64>,
    capped: bool,
    capture: Option<CaptureEvent>,
}

/// In-memory, instance-owned stage ring and monotonic elapsed-time origin.
/// Retains at most MAX_EVENTS, clamps durations to 24 hours and accepts no
/// freeform messages. Dropping it discards its events; nothing is persisted.
#[derive(Clone)]
pub struct Diagnostics {
    started: Instant,
    events: VecDeque<Event>,
    discarded: u64,
}
impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl Diagnostics {
    /// Starts an empty independent ring with elapsed time relative to now.
    /// No settings, log contents, files or device state are read.
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            events: VecDeque::new(),
            discarded: 0,
        }
    }

    /// Records stage timing only; callers cannot pass a message or an ID.
    /// Evicts the oldest record at MAX_EVENTS and clamps duration to 24 hours.
    pub fn record(&mut self, phase: Phase, outcome: Outcome, duration_ms: Option<u64>) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
            self.discarded = self.discarded.saturating_add(1);
        }
        self.events.push_back(Event {
            elapsed_ms: self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            phase,
            outcome,
            duration_ms: duration_ms.map(|value| value.min(MAX_DURATION_MS)),
            capped: duration_ms.is_some_and(|value| value > MAX_DURATION_MS),
            capture: None,
        });
    }

    /// Bounded capture timing records contain only an app-run ordinal and enums.
    pub fn record_capture(&mut self, mut capture: CaptureEvent) {
        if let Some(duration) = capture.duration_ms {
            capture.capped |= duration > MAX_DURATION_MS;
            capture.duration_ms = Some(duration.min(MAX_DURATION_MS));
        }
        capture.attempt = capture.attempt.filter(|ordinal| *ordinal != 0);
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
            self.discarded = self.discarded.saturating_add(1);
        }
        self.events.push_back(Event {
            elapsed_ms: self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
            phase: Phase::CaptureTiming,
            outcome: match capture.outcome {
                CaptureOutcome::Failed => Outcome::Failed,
                CaptureOutcome::Cancelled => Outcome::Cancelled,
                CaptureOutcome::Unavailable => Outcome::Unavailable,
                CaptureOutcome::Started => Outcome::Started,
                _ => Outcome::Ok,
            },
            duration_ms: capture.duration_ms,
            capped: capture.capped,
            capture: Some(capture),
        });
    }

    /// Creates an owned snapshot with a fresh opaque ID and the exact export
    /// text. Context is copied into allowlisted fields; invalid version text is
    /// replaced. Events remain in this ring and the returned snapshot is fixed.
    pub fn report(&self, context: ReportContext<'_>) -> Report {
        let sequence = REPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tick = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!("{:016x}-{:08x}", tick as u64, sequence as u32);
        // All other field values are finite enums/booleans/bounded numbers.
        // Credential fields cannot enter this renderer's fixed field set.
        let version =
            privacy::sanitize_field("version", context.version, privacy::Encoding::Metadata)
                .unwrap_or_else(|| "unavailable".into());
        let mut text = format!(
            "notype diagnostic report v1\nreport_id={id}\nversion={version}\nplatform={}\nlog_level={}\ndebug_mode={}\nalways_on_microphone={}\nmodel_selected={}\nmodel_available={}\nmodel_is_custom={}\nlocal_log={}\nevents_retained={}\nevents_discarded={}\n\nEmpty logs do not establish that no errors occurred.\nOnly bounded stage metadata is included. Raw logs, audio, transcripts, text results, endpoints, secrets, paths and device identifiers are excluded.\nEvents are from this app run only; elapsed_ms is relative to diagnostic initialization.\n\n",
            context.platform.label(), context.log_level.label(), context.debug_mode,
            context.always_on_microphone, context.model_selected, context.model_available,
            context.model_is_custom, context.local_log.label(), self.events.len(), self.discarded,
        );
        for event in &self.events {
            let _ = write!(
                text,
                "elapsed_ms={} phase={} outcome={}",
                event.elapsed_ms,
                event.phase.label(),
                event.outcome.label()
            );
            if let Some(duration) = event.duration_ms {
                let _ = write!(text, " duration_ms={duration}");
            }
            let _ = write!(text, " capped={}", event.capped);
            if let Some(capture) = &event.capture {
                let _ = write!(
                    text,
                    " attempt={} span={} origin={} capture_outcome={}",
                    capture
                        .attempt
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "unassociated".into()),
                    capture.span.label(),
                    capture.span.origin(),
                    capture.outcome.label()
                );
            }
            let code = match event.outcome {
                Outcome::Failed => format!("{}-failed", event.phase.label()),
                Outcome::Unavailable => format!("{}-unavailable", event.phase.label()),
                _ => "none".into(),
            };
            let _ = writeln!(text, " code={code}");
        }
        let _ = writeln!(text, "\nring_truncated={}", self.discarded > 0);
        let mut attempts = BTreeMap::<u32, (bool, bool, bool, Vec<CaptureSpan>)>::new();
        for event in &self.events {
            if let Some(capture) = &event.capture {
                if let Some(ordinal) = capture.attempt {
                    let entry = attempts.entry(ordinal).or_default();
                    if capture.span == CaptureSpan::Attempt {
                        entry.0 |= capture.outcome == CaptureOutcome::Started;
                        entry.1 |= capture.outcome.terminal();
                    }
                    entry.2 |= capture.capped;
                    if capture.span != CaptureSpan::Attempt {
                        if capture.outcome == CaptureOutcome::Started
                            && capture.duration_ms.is_none()
                        {
                            if !entry.3.contains(&capture.span) {
                                entry.3.push(capture.span);
                            }
                        } else if capture.outcome != CaptureOutcome::Started {
                            entry.3.retain(|span| *span != capture.span);
                        }
                    }
                }
            }
        }
        for (ordinal, (started, terminal, capped, pending)) in attempts {
            let complete = started && terminal && pending.is_empty();
            let _ = writeln!(text, "attempt_summary={} complete={} incomplete={} truncated={} capped={} pending_stages={}", ordinal, complete, !complete, !started || (self.discarded > 0 && !terminal), capped, pending.len());
        }
        text.push_str("Timing durations have explicit origins and may overlap; do not sum cumulative and local spans. ShortcutAccepted is worker-observed, not physical acceptance. Consumer sample observation is after the audio ring; raw hardware callback timing is unavailable. Frontend round-trip includes return IPC; React commit is not paint. Audible cue timing is unavailable.\n");
        Report { id, text }
    }
}

/// Shared app-run collector facade. First use creates an uninitialized/OFF
/// service; records remain dropped until preference initialization enables it.
/// Only typed codes and a bounded duration can cross this boundary.
pub fn record(phase: Phase, outcome: Outcome, duration_ms: Option<u64>) {
    service::global().record(phase, outcome, duration_ms);
}
