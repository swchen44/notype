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
    /// Fixed official Flash block with conservative monetary reservation.
    DeepSeek,
    /// Fixed free Gemma block through the sole permitted route.
    OpenRouter,
    /// Label only; no live factory or credential path exists.
    GeminiSealed,
}
/// Purpose of a fixed authorized slot, independent of a later setting.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Purpose {
    /// Exact OK Custom connection check, one real observed call.
    Connection,
    /// Chinese cleanup preserving identifiers/numbers/negation.
    Dictation,
    /// Fixed English translation without automatic review.
    Translation,
    /// Fixed Chinese notification drafting scene.
    Ask,
    /// Forward translation requiring the scheduled actual reverse.
    ForwardReview,
    /// Automatic reverse using actual forward output on the same history.
    Reverse,
    /// Slot7 request with a joined owned cancellation timer.
    Cancellation,
    /// Slot8 explicit retry on the same history, never a transport retry.
    Retry,
    /// Permanently ineligible comparison entry.
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
// These probes recognise only a small set of fixed-corpus forms. Normalising a
// local copy never edits stored output and never certifies general meaning.
fn compact(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| match character {
            '，' => ',',
            '；' => ';',
            '。' => '.',
            '’' => '\'',
            other => other,
        })
        .collect::<String>()
        .to_ascii_lowercase()
}
fn fact_range(text: &str, phrase: &str) -> Option<std::ops::Range<usize>> {
    text.match_indices(phrase).find_map(|(start, _)| {
        let end = start + phrase.len();
        text[end..]
            .chars()
            .next()
            .is_none_or(|next| !next.is_ascii_alphanumeric() && next != '_')
            .then_some(start..end)
    })
}
fn protected_negation(text: &str, positive: &[String], negative: &[String]) -> bool {
    let mut remaining = text.to_owned();
    let mut found = false;
    for phrase in negative {
        while let Some(range) = fact_range(&remaining, phrase) {
            remaining.replace_range(range, " ");
            found = true;
        }
    }
    found
        && !positive
            .iter()
            .any(|phrase| fact_range(&remaining, phrase).is_some())
}
fn prohibits_deletion(text: &str, identifier: &str) -> bool {
    let mut positive = vec![];
    let mut negative = vec![];
    for verb in ["delete", "deletethe", "remove", "removethe"] {
        let phrase = format!("{verb}{identifier}");
        for prefix in ["donot", "don't", "mustnot"] {
            negative.push(format!("{prefix}{phrase}"));
        }
        positive.push(phrase);
    }
    for verb in ["刪掉", "刪除"] {
        let phrase = format!("{verb}{identifier}");
        for prefix in ["不要", "請勿", "不得", "不能"] {
            negative.push(format!("{prefix}{phrase}"));
        }
        positive.push(phrase);
    }
    protected_negation(text, &positive, &negative)
}
fn not_five_seconds(text: &str) -> bool {
    [
        "not5seconds",
        "not5second",
        "不是5秒",
        "而非5秒",
        "0.5seconds,not5",
        "0.5second,not5",
        "0.5,not5",
        "0.5秒,不是5",
        "0.5,不是5",
    ]
    .iter()
    .any(|phrase| {
        fact_range(text, phrase).is_some_and(|range| {
            let mut tail = text[range.end..].chars();
            !(tail.next() == Some('.') && tail.next().is_some_and(|next| next.is_ascii_digit()))
        })
    })
}
fn full_corpus(text: &str, probe: &str) -> bool {
    ["session_id", "timeout", "0.5", "42", "API"]
        .iter()
        .all(|part| text.contains(part))
        && prohibits_deletion(probe, "session_id")
        && not_five_seconds(probe)
}
fn no_slide_preparation(text: &str) -> bool {
    let positive = vec!["準備投影片".into(), "prepareslides".into()];
    let mut negative = ["不用", "不必", "無須", "無需", "不需", "不要", "請勿"]
        .iter()
        .map(|prefix| format!("{prefix}準備投影片"))
        .collect::<Vec<_>>();
    negative.extend(
        [
            "donotprepareslides",
            "don'tprepareslides",
            "noneedtoprepareslides",
        ]
        .map(str::to_owned),
    );
    protected_negation(text, &positive, &negative)
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
    let chinese = text.chars().any(|c| matches!(c as u32, 0x3400..=0x9fff));
    let english_target = matches!(slot.purpose, Purpose::Translation | Purpose::ForwardReview);
    if slot.purpose != Purpose::Connection
        && (chinese == english_target
            || ["here is", "here's", "以下是", "以下為", "這是翻譯", "```"]
                .iter()
                .any(|prefix| text.to_ascii_lowercase().starts_with(prefix)))
    {
        return false;
    }
    let probe = compact(text);
    match slot.purpose {
        Purpose::Connection => text == "OK",
        Purpose::Dictation | Purpose::Translation => full_corpus(text, &probe),
        Purpose::ForwardReview | Purpose::Reverse if slot.provider == Provider::DeepSeek => {
            ["API_42", "timeout", "0.5"]
                .iter()
                .all(|part| text.contains(part))
                && prohibits_deletion(&probe, "api_42")
        }
        Purpose::ForwardReview | Purpose::Reverse => full_corpus(text, &probe),
        Purpose::Ask => {
            (text.contains("星期三") || text.to_ascii_lowercase().contains("wednesday"))
                && (text.contains("兩點") || text.contains('2'))
                && no_slide_preparation(&probe)
        }
        Purpose::Cancellation | Purpose::Retry => {
            text.chars().any(|c| matches!(c as u32, 0x3400..=0x9fff))
        }
        Purpose::Sealed => false,
    }
}
