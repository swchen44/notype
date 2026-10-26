//! Immutable P25-live-v4 schedule and conservative authorization, never billing.
use notype_core::Scenario;
use serde::{Deserialize, Serialize};
/// One immutable round. Existing or partial state can only be reported.
pub const ROUND_ID: &str = "P25-live-v4";
/// Total authorized USD in units of 10^-7. This is a ceiling, not a target.
pub const BUDGET_UNITS: u64 = 20_000_000;
/// Conservative DeepSeek per-call reservation; unknown outcomes never refund it.
pub const DEEPSEEK_RESERVATION: u64 = 104_448;
/// Maximum eligible inference calls, including retry and automatic reverse.
pub const MAX_CALLS: usize = 12;
/// Fixed corpus, no runtime user/transcript input.
pub const DICTATION: &str =
    "那個我們不要刪掉 session_id，timeout 設成 0.5 秒，不是 5 秒，保留 42 個 API request。";
/// Fixed review sample, independently checked after real forward/reverse calls.
pub const REVIEW: &str = "不要刪除 API_42，請把 timeout 保留在 0.5 秒。";
/// Fixed cancellation/retry sample; retry never changes the original.
pub const CANCELLATION: &str = "用繁體中文寫三句關於測試資料的短文，不要引用外部資料。";
/// Fixed synthetic instruction sample.
pub const ASK: &str = "幫我寫一句通知：下週會議改到星期三下午兩點，不用準備投影片。";
/// Public provider identity; Gemini exists solely as a sealed schedule label.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Provider {
    DeepSeek,
    OpenRouter,
    GeminiSealed,
}
/// Purpose of a fixed authorized slot, independent of a later setting.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Purpose {
    Connection,
    Dictation,
    Translation,
    Ask,
    ForwardReview,
    Reverse,
    Cancellation,
    Retry,
    Sealed,
}
/// Immutable nonsecret slot contract. It never stores a key or actual fee.
#[derive(Clone, Copy)]
pub struct Slot {
    /// Fixed one-based position in the sixteen-slot manifest.
    pub number: u8,
    /// Histories shared only by a scheduled reverse or explicit retry.
    pub group: u8,
    /// Exact provider block; Gemini has no eligible request.
    pub provider: Provider,
    /// Required generation/reverse/cancel purpose.
    pub purpose: Purpose,
    /// Fixed original input. Reverse itself uses the actual forward result.
    pub corpus: &'static str,
}
impl Slot {
    /// Existing production scene, without synthetic microphone metadata.
    pub fn scenario(self) -> Scenario {
        match self.purpose {
            Purpose::Dictation => Scenario::Dictation,
            Purpose::Translation | Purpose::ForwardReview | Purpose::Reverse => {
                Scenario::Translation
            }
            _ => Scenario::Ask,
        }
    }
    /// Conservative authorization, not an actual reported charge.
    pub fn reservation(self) -> u64 {
        if self.provider == Provider::DeepSeek {
            DEEPSEEK_RESERVATION
        } else {
            0
        }
    }
    /// Requested identity is separate from provider-reported identity.
    pub fn model(self) -> &'static str {
        match self.provider {
            Provider::DeepSeek => "deepseek-flash",
            Provider::OpenRouter => "google/gemma-4-31b-it:free",
            Provider::GeminiSealed => "gemini-3.8-flash",
        }
    }
}
/// Complete immutable manifest. All four Gemini entries seal at arm forever.
pub fn schedule() -> [Slot; 16] {
    use Provider::*;
    use Purpose::*;
    let make = |number, group, provider, purpose, corpus| Slot {
        number,
        group,
        provider,
        purpose,
        corpus,
    };
    [
        make(1, 1, DeepSeek, Connection, "只回答 OK。"),
        make(2, 2, DeepSeek, Dictation, DICTATION),
        make(3, 3, DeepSeek, Translation, DICTATION),
        make(4, 4, DeepSeek, Ask, ASK),
        make(5, 5, DeepSeek, ForwardReview, REVIEW),
        make(6, 5, DeepSeek, Reverse, REVIEW),
        make(7, 6, DeepSeek, Cancellation, CANCELLATION),
        make(8, 6, DeepSeek, Retry, CANCELLATION),
        make(9, 7, OpenRouter, Dictation, DICTATION),
        make(10, 8, OpenRouter, ForwardReview, DICTATION),
        make(11, 8, OpenRouter, Reverse, DICTATION),
        make(12, 9, OpenRouter, Ask, ASK),
        make(13, 10, GeminiSealed, Sealed, DICTATION),
        make(14, 11, GeminiSealed, Sealed, DICTATION),
        make(15, 11, GeminiSealed, Sealed, DICTATION),
        make(16, 12, GeminiSealed, Sealed, ASK),
    ]
}
/// Bounded local automated checks, not certification of human meaning.
pub fn quality(slot: Slot, text: &str) -> bool {
    let text = text.trim();
    if text.is_empty()
        || text.len() > 16_384
        || text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
    {
        return false;
    }
    match slot.purpose {
        Purpose::Connection => text == "OK",
        Purpose::Dictation | Purpose::Translation => {
            ["session_id", "0.5", "42", "API"]
                .iter()
                .all(|part| text.contains(part))
                && (text.contains('不')
                    || text.to_ascii_lowercase().contains("not")
                    || text.to_ascii_lowercase().contains("don't"))
        }
        Purpose::ForwardReview | Purpose::Reverse if slot.provider == Provider::DeepSeek => {
            ["API_42", "timeout", "0.5"]
                .iter()
                .all(|part| text.contains(part))
                && (text.contains('不')
                    || text.to_ascii_lowercase().contains("not")
                    || text.to_ascii_lowercase().contains("don't"))
        }
        Purpose::ForwardReview | Purpose::Reverse => ["session_id", "0.5", "42", "API"]
            .iter()
            .all(|part| text.contains(part)),
        Purpose::Ask => {
            (text.contains("星期三") || text.to_ascii_lowercase().contains("wednesday"))
                && (text.contains("兩點") || text.contains('2'))
        }
        Purpose::Cancellation | Purpose::Retry => {
            text.chars().any(|c| matches!(c as u32, 0x3400..=0x9fff))
        }
        Purpose::Sealed => false,
    }
}
