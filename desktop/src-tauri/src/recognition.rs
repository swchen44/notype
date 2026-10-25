//! Local recognition profiles. No platform handles, UI locale, or network calls.
use notype_core::RecognitionMode;
use opencc_jieba_rs::OpenCC;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug)]
pub struct RecognitionFailure;

pub const CPU_THREADS: i32 = 4;

#[derive(Clone, Debug)]
pub struct RecognitionProfile {
    pub mode: RecognitionMode,
    pub protected_terms: Vec<String>,
}

impl RecognitionProfile {
    pub fn new(mode: RecognitionMode, protected_terms: Vec<String>) -> Self {
        Self {
            mode,
            protected_terms,
        }
    }

    pub fn normalize(&self, raw: &str) -> Result<String, RecognitionFailure> {
        normalize_transcript(raw, self.mode, &self.protected_terms)
    }

    /// Both writing profiles need one multilingual recognizer, not an English-only model.
    pub fn validate_model(&self, languages: &[String]) -> Result<(), &'static str> {
        let supports = |wanted: &str| {
            languages.iter().any(|code| {
                code.split(['-', '_'])
                    .next()
                    .is_some_and(|base| base.eq_ignore_ascii_case(wanted))
            })
        };
        if supports("zh") && supports("en") {
            Ok(())
        } else {
            Err("recognition-model-incompatible")
        }
    }
}

/// Convert the recognized original locally, before persistence or text processing.
fn normalize_transcript(
    raw: &str,
    mode: RecognitionMode,
    protected_terms: &[String],
) -> Result<String, RecognitionFailure> {
    static CONVERTER: OnceLock<Result<OpenCC, RecognitionFailure>> = OnceLock::new();
    let converter = CONVERTER
        .get_or_init(|| std::panic::catch_unwind(OpenCC::new).map_err(|_| RecognitionFailure))
        .as_ref()
        .map_err(|_| RecognitionFailure)?;
    static LITERALS: OnceLock<Result<Regex, RecognitionFailure>> = OnceLock::new();
    let literals = LITERALS.get_or_init(|| Regex::new(
        r###"(?s:```.*?(?:```|\z))|`[^`\r\n]*(?:`|\z)|"(?:\\.|[^"\\])*"|(?:https?://|www\.)[^\s<>"`，。！？；]+|[A-Za-z]:[\\/][^\s"`，。！？；]+|[A-Za-z0-9_.+-]+@[\p{L}\p{N}.-]+\.[A-Za-z]{2,}|[A-Za-z_$][A-Za-z0-9_$]*(?:_[\p{L}\p{N}]+)+"###,
    ).map_err(|_| RecognitionFailure)).as_ref().map_err(|_| RecognitionFailure)?;
    let mut spans: Vec<_> = literals
        .find_iter(raw)
        .map(|m| (m.start(), m.end()))
        .collect();
    for term in protected_terms.iter().filter(|term| !term.is_empty()) {
        spans.extend(
            raw.match_indices(term)
                .map(|(start, term)| (start, start + term.len())),
        );
    }
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        if let Some(last) = merged.last_mut().filter(|last| start <= last.1) {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let convert = |text: &str| match mode {
            RecognitionMode::TraditionalEnglish => converter.s2tw(text, false),
            RecognitionMode::SimplifiedEnglish => converter.tw2s(text, false),
        };
        let mut output = String::new();
        let mut cursor = 0;
        for (start, end) in merged {
            output.push_str(&convert(&raw[cursor..start]));
            output.push_str(&raw[start..end]);
            cursor = end;
        }
        output.push_str(&convert(&raw[cursor..]));
        output
    }))
    .map_err(|_| RecognitionFailure)
}
