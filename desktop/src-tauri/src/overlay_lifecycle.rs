//! Capture identity and presentation revisions, independent of native window APIs.
//! Reserve before queueing work; commit under the same registry lock as reserve.

/// Ordered presentation stages for one capture; stream variants retain its UI mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    /// Audio capture with the compact overlay.
    Recording,
    /// Audio capture with streaming text presentation.
    Streaming,
    /// Captured audio is being transcribed.
    Transcribing,
    /// Recognized text is being processed.
    Processing,
    /// Streaming presentation retained while transcription finishes.
    StreamTranscribing,
    /// Streaming presentation retained while text processing finishes.
    StreamProcessing,
    /// Terminal presentation; the same capture cannot be shown again.
    Hidden,
}
impl Presentation {
    /// Whether this stage can accept an exact-ticket microphone-ready event.
    pub fn capturing(self) -> bool {
        matches!(self, Self::Recording | Self::Streaming)
    }
    /// Whether the adapter should retain streaming geometry and presentation.
    pub fn streaming(self) -> bool {
        matches!(
            self,
            Self::Streaming | Self::StreamTranscribing | Self::StreamProcessing
        )
    }
    /// Stable frontend state label; all streaming stages share `streaming`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Streaming | Self::StreamTranscribing | Self::StreamProcessing => "streaming",
            Self::Transcribing => "transcribing",
            Self::Processing => "processing",
            Self::Hidden => "hidden",
        }
    }
    fn rank(self) -> u8 {
        match self {
            Self::Recording | Self::Streaming => 0,
            Self::Transcribing | Self::StreamTranscribing => 1,
            Self::Processing | Self::StreamProcessing => 2,
            Self::Hidden => 3,
        }
    }
}

/// Terminal reasons are a closed set, never raw error messages.
#[derive(Clone, Copy, Debug)]
pub enum HideReason {
    // Reserved by the public hide contract for a direct overlay close action.
    #[allow(dead_code)]
    /// An explicit overlay close request.
    Closed,
    /// The workflow completed normally.
    Finished,
    /// The capture or workflow was cancelled.
    Cancelled,
    /// The workflow failed; no raw error text is carried by this reason.
    Failed,
}
impl HideReason {
    /// Stable frontend reason label, independent of private error details.
    pub fn label(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Finished => "finished",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// A process-local workflow generation, never a persistent text/history ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaptureId(
    /// Monotonically ordered workflow generation within this process.
    pub u64,
);

/// Immutable request identity, captured before asynchronous/native dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayTicket {
    /// Workflow identity shared by every presentation for this capture.
    pub capture: CaptureId,
    /// Registry-wide presentation revision; distinct from capture identity.
    pub revision: u64,
    /// Presentation reserved by this exact ticket.
    pub presentation: Presentation,
}

/// Fences queued native/UI work against newer captures and presentation changes.
///
/// The native adapter owns synchronization: use one registry lock for reserve,
/// then reacquire it and hold it through validation and the synchronous commit
/// closure. This policy has no internal lock and cannot protect an adapter that
/// checks and acts separately.
pub struct OverlayLifecycle {
    current: Option<OverlayTicket>,
    next_revision: u64,
}
impl OverlayLifecycle {
    /// Creates an empty process-local registry with no active presentation.
    pub const fn new() -> Self {
        Self {
            current: None,
            next_revision: 0,
        }
    }
    /// Reserves a presentation before its asynchronous/native work is queued.
    ///
    /// Duplicate show is idempotent. Older captures and backwards stages are
    /// rejected; a terminal capture cannot be rearmed. The adapter must use its
    /// registry lock here and again around the eventual check/commit operation.
    pub fn reserve(
        &mut self,
        capture: CaptureId,
        presentation: Presentation,
    ) -> Option<OverlayTicket> {
        if let Some(current) = self.current {
            if capture < current.capture {
                return None;
            }
            if capture == current.capture {
                if presentation == current.presentation {
                    return Some(current);
                }
                if presentation.rank() < current.presentation.rank()
                    || current.presentation == Presentation::Hidden
                {
                    return None;
                }
            }
        }
        self.next_revision = self.next_revision.checked_add(1)?;
        let ticket = OverlayTicket {
            capture,
            revision: self.next_revision,
            presentation,
        };
        self.current = Some(ticket);
        Some(ticket)
    }
    /// Runs an operation only while its exact capture/presentation ticket is current.
    ///
    /// The adapter must hold the registry lock throughout this check and closure,
    /// using the same lock as `reserve`, so queued stale work cannot hide/show a
    /// successor between validation and its native operation.
    pub fn commit<T>(&self, ticket: OverlayTicket, operation: impl FnOnce() -> T) -> Option<T> {
        (self.current == Some(ticket)).then(operation)
    }
    /// Runs a ready notification only for the exact current capturing ticket.
    ///
    /// Both capture identity and presentation revision must still match; a
    /// transition to working/hidden invalidates ready. Hold the adapter's registry
    /// lock through validation and the notification closure, as for `commit`.
    pub fn ready<T>(&self, ticket: OverlayTicket, operation: impl FnOnce() -> T) -> Option<T> {
        if !ticket.presentation.capturing() {
            return None;
        }
        self.commit(ticket, operation)
    }
    /// Validates a read-only observation against the exact current capturing envelope.
    ///
    /// Rejects wrong capture/revision, working/hidden stages and superseded ACKs
    /// without running their closure. Settings observations also rely on this
    /// outer revision gate before entering correlation/deduplication. Hold the
    /// adapter's registry lock through this check and observation closure.
    pub fn observe<T>(
        &self,
        capture: CaptureId,
        revision: u64,
        operation: impl FnOnce() -> T,
    ) -> Option<T> {
        self.current
            .filter(|current| {
                current.capture == capture
                    && current.revision == revision
                    && current.presentation.capturing()
            })
            .map(|_| operation())
    }
    /// Runs text delivery for the current capture until its presentation is hidden.
    ///
    /// Unlike `ready`, text intentionally survives presentation revisions and
    /// remains valid during nonhidden transcription/processing stages. Newer
    /// captures and terminal hide reject it. Hold the adapter's registry lock
    /// through this check and delivery closure.
    pub fn text<T>(&self, capture: CaptureId, operation: impl FnOnce() -> T) -> Option<T> {
        self.current
            .filter(|current| {
                current.capture == capture && current.presentation != Presentation::Hidden
            })
            .map(|_| operation())
    }
}

impl Default for OverlayLifecycle {
    fn default() -> Self {
        Self::new()
    }
}
