//! Privacy boundary for support reports. Only typed, bounded metadata enters
//! this module. It never reads raw logs, settings documents, history or audio.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const MAX_EVENTS: usize = 128;
const MAX_DURATION_MS: u64 = 86_400_000;
static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static EVENTS: OnceLock<Mutex<Diagnostics>> = OnceLock::new();

macro_rules! labels {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }
        }
    };
}

labels!(Phase {
    Startup => "startup",
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
labels!(Outcome {
    Started => "started", Ok => "ok", Failed => "failed",
    Cancelled => "cancelled", Unavailable => "unavailable",
});
labels!(Platform { Windows => "windows", Macos => "macos", Linux => "linux", Other => "other" });
labels!(LogLevel { Error => "error", Warn => "warn", Info => "info", Debug => "debug", Trace => "trace" });
labels!(LogFileStatus { Empty => "empty", Present => "present", Missing => "missing", Unavailable => "unavailable" });

/// All fields except version are enums or booleans. Version is restricted to a
/// short numeric release identifier at the report boundary.
pub struct ReportContext<'a> {
    pub version: &'a str,
    pub platform: Platform,
    pub log_level: LogLevel,
    pub debug_mode: bool,
    pub always_on_microphone: bool,
    pub model_selected: bool,
    pub model_available: bool,
    pub model_is_custom: bool,
    pub local_log: LogFileStatus,
}

#[derive(Clone)]
pub struct Report {
    id: String,
    text: String,
}

impl Report {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Saves the immutable, previously reviewed snapshot. Never returns a path
    /// or an OS error message; the native adapter owns the explicit dialog.
    pub fn export_to(&self, path: &Path) -> Result<(), &'static str> {
        if path.extension().and_then(|extension| extension.to_str()) != Some("txt") {
            return Err("diagnostic-extension-invalid");
        }
        std::fs::write(path, self.text.as_bytes()).map_err(|_| "diagnostic-save-failed")
    }
}

struct Event {
    elapsed_ms: u64,
    phase: Phase,
    outcome: Outcome,
    duration_ms: Option<u64>,
}

pub struct Diagnostics {
    started: Instant,
    events: VecDeque<Event>,
    discarded: u64,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            events: VecDeque::new(),
            discarded: 0,
        }
    }

    /// Records stage timing only; callers cannot pass a message or an ID.
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
        });
    }

    /// Creates the exact text shown in Preview and subsequently saved to disk.
    pub fn report(&self, context: ReportContext<'_>) -> Report {
        let sequence = REPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tick = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!("{:016x}-{:08x}", tick as u64, sequence as u32);
        let version = safe_version(context.version);
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
            let code = match event.outcome {
                Outcome::Failed => format!("{}-failed", event.phase.label()),
                Outcome::Unavailable => format!("{}-unavailable", event.phase.label()),
                _ => "none".into(),
            };
            let _ = writeln!(text, " code={code}");
        }
        Report { id, text }
    }
}

fn safe_version(version: &str) -> &str {
    if !version.is_empty()
        && version.len() <= 32
        && version
            .bytes()
            .all(|value| value.is_ascii_digit() || value == b'.')
        && version.bytes().any(|value| value.is_ascii_digit())
    {
        version
    } else {
        "unavailable"
    }
}

/// Global app-run event ring. A poisoned lock cannot panic the product path.
pub fn record(phase: Phase, outcome: Outcome, duration_ms: Option<u64>) {
    let mut diagnostics = EVENTS
        .get_or_init(|| Mutex::new(Diagnostics::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    diagnostics.record(phase, outcome, duration_ms);
}

pub fn report(context: ReportContext<'_>) -> Report {
    EVENTS
        .get_or_init(|| Mutex::new(Diagnostics::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .report(context)
}
