//! Fixed, nonsecret admission policy for the single approved DeepSeek trial.
//! No HTTP, credentials, App state or history fee calculation belongs here.

/// USD units of 10^-7 reserved per request under the reviewed operational bound.
pub const RESERVATION_UNITS: u64 = 104_448;
/// User-authorized USD 0.10, expressed in units of 10^-7.
pub const BUDGET_UNITS: u64 = 1_000_000;
/// Fixed identity; reopening this round never starts another paid round.
pub const ROUND_ID: &str = "P23-live-v1";

/// Safe facts from the *complete actual wire*. Never pass credentials or text.
/// The future production bridge must obtain these after request construction.
pub struct WireFacts<'a> {
    /// Borrowed final endpoint; must be the pinned public URL without credentials.
    pub endpoint: &'a str,
    /// Borrowed model identifier actually serialized in the request body.
    pub model: &'a str,
    /// Whether the request used the explicit reviewed DeepSeek profile.
    pub deepseek_profile: bool,
    /// Explicit final effort value; absence is not equivalent to the required value.
    pub effort: Option<&'a str>,
    /// Explicit final thinking flag; absence cannot silently inherit a default.
    pub thinking: Option<bool>,
    /// Whether the actual request uses the approved nonstreaming response mode.
    pub non_streaming: bool,
    /// Output-token limit actually present on the wire, not a UI preference.
    pub max_output_tokens: Option<u32>,
    /// Complete serialized request size, including every field, in bytes.
    pub wire_bytes: u64,
    /// Number of messages in that complete request, including system messages.
    pub message_count: u32,
    /// Whether all message content uses plain text without tools or attachments.
    pub plain_text_only: bool,
}

/// Fixed codes contain no configuration, request text or secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    /// Final wire facts violate the pinned provider/request contract.
    WireContract,
    /// Slot, state or local identifier violates the fixed one-round schedule.
    Schedule,
    /// The fresh round has not been durably armed.
    NotArmed,
    /// Existing evidence was reopened; no mutation or send may be authorized.
    ReportOnly,
    /// A prior halt or storage failure blocks further admission in this process.
    Halted,
    /// Storage is missing, corrupt, inconsistent or failed; never includes raw details.
    Storage,
    /// Another process or handle currently owns the exclusive round lock.
    AlreadyOwned,
}

/// Purpose of one fixed live slot. Never infer it from a later user setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The single initial connection request.
    Connection,
    /// Fixed synthetic dictation sample.
    Dictation,
    /// Translation of the fixed synthetic sample.
    Translation,
    /// Fixed synthetic instruction request.
    Ask,
    /// Forward result for the scheduled translation review.
    ForwardReview,
    /// Optional reverse call sharing the forward result's history group.
    BackTranslation,
    /// Scheduled cancellation attempt; its reservation is never refunded.
    Cancellation,
    /// One scheduled retry sharing slot 7's history, with its own call ID.
    Retry,
}

/// Exact nonsecret schedule item, shared by the eventual runner and guard.
pub struct SlotSpec {
    /// Unique fixed position from 1 through 8; no replacement slots are created.
    pub number: u8,
    /// Required local history grouping, independent of call IDs or provider IDs.
    pub history_group: u8,
    /// Immutable purpose of this authorized slot.
    pub purpose: Purpose,
    /// Reviewed static synthetic input; never user speech or a credential.
    pub sample: &'static str,
    /// Whether this slot belongs to the scheduled forward/reverse review pair.
    pub review: bool,
}

/// Return the fixed eight-slot schedule; slots 5/6 and 7/8 share history IDs.
pub fn schedule() -> &'static [SlotSpec] {
    const DICTATION: &str =
        "那個我們不要刪掉 session_id，timeout 設成 0.5 秒，不是 5 秒，保留 42 個 API request。";
    const CANCEL: &str = "用繁體中文寫三句關於測試資料的短文，不要引用外部資料。";
    &[
        SlotSpec {
            number: 1,
            history_group: 1,
            purpose: Purpose::Connection,
            sample: "只回答 OK。",
            review: false,
        },
        SlotSpec {
            number: 2,
            history_group: 2,
            purpose: Purpose::Dictation,
            sample: DICTATION,
            review: false,
        },
        SlotSpec {
            number: 3,
            history_group: 3,
            purpose: Purpose::Translation,
            sample: DICTATION,
            review: false,
        },
        SlotSpec {
            number: 4,
            history_group: 4,
            purpose: Purpose::Ask,
            sample: "幫我寫一句通知：下週會議改到星期三下午兩點，不用準備投影片。",
            review: false,
        },
        SlotSpec {
            number: 5,
            history_group: 5,
            purpose: Purpose::ForwardReview,
            sample: "不要刪除 API_42，請把 timeout 保留在 0.5 秒。",
            review: true,
        },
        SlotSpec {
            number: 6,
            history_group: 5,
            purpose: Purpose::BackTranslation,
            sample: "",
            review: true,
        },
        SlotSpec {
            number: 7,
            history_group: 6,
            purpose: Purpose::Cancellation,
            sample: CANCEL,
            review: false,
        },
        SlotSpec {
            number: 8,
            history_group: 6,
            purpose: Purpose::Retry,
            sample: CANCEL,
            review: false,
        },
    ]
}

/// Enforce the approved profile and complete serialized-wire bound.
pub fn validate_wire(facts: &WireFacts<'_>) -> Result<(), Denied> {
    if facts.endpoint != "https://api.deepseek.com/chat/completions" {
        return Err(Denied::WireContract);
    }
    validate_controls(facts)
}

pub(super) fn validate_controls(facts: &WireFacts<'_>) -> Result<(), Denied> {
    if facts.model != "deepseek-flash"
        || !facts.deepseek_profile
        || facts.effort != Some("none")
        || facts.thinking != Some(false)
        || !facts.non_streaming
        || facts.max_output_tokens != Some(512)
        || !(1..=8192).contains(&facts.wire_bytes)
        || !(1..=4).contains(&facts.message_count)
        || !facts.plain_text_only
    {
        return Err(Denied::WireContract);
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn validate_fixture_wire(facts: &WireFacts<'_>) -> Result<(), Denied> {
    let port = facts
        .endpoint
        .strip_prefix("http://127.0.0.1:")
        .and_then(|suffix| suffix.strip_suffix("/chat/completions"))
        .and_then(|port| port.parse::<u16>().ok())
        .filter(|port| *port > 0);
    if port.is_none() {
        return Err(Denied::WireContract);
    }
    validate_controls(facts)
}
