//! Capture identity and presentation revisions, independent of native window APIs.
//! Reserve before queueing work; commit under the same registry lock as reserve.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presentation {
    Recording,
    Streaming,
    Transcribing,
    Processing,
    StreamTranscribing,
    StreamProcessing,
    Hidden,
}
impl Presentation {
    pub fn capturing(self) -> bool {
        matches!(self, Self::Recording | Self::Streaming)
    }
    pub fn streaming(self) -> bool {
        matches!(
            self,
            Self::Streaming | Self::StreamTranscribing | Self::StreamProcessing
        )
    }
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
    Closed,
    Finished,
    Cancelled,
    Failed,
}
impl HideReason {
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
pub struct CaptureId(pub u64);

/// Immutable request identity, captured before asynchronous/native dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayTicket {
    pub capture: CaptureId,
    pub revision: u64,
    pub presentation: Presentation,
}

pub struct OverlayLifecycle {
    current: Option<OverlayTicket>,
    next_revision: u64,
}
impl OverlayLifecycle {
    pub const fn new() -> Self {
        Self {
            current: None,
            next_revision: 0,
        }
    }
    /// Duplicate show is idempotent. A terminal capture cannot be rearmed.
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
    /// Commit while the adapter holds the registry lock; no check-then-hide race.
    pub fn commit<T>(&self, ticket: OverlayTicket, operation: impl FnOnce() -> T) -> Option<T> {
        (self.current == Some(ticket)).then(operation)
    }
    pub fn ready<T>(&self, ticket: OverlayTicket, operation: impl FnOnce() -> T) -> Option<T> {
        if !ticket.presentation.capturing() {
            return None;
        }
        self.commit(ticket, operation)
    }
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
