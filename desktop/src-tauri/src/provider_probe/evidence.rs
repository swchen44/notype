//! Fixed unauthenticated contract observations. This is not an inference client;
//! it has no credential, URL override, account lookup or retry operation.
use super::{
    journal::{Failure, Identity},
    native, policy,
};
use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PAGES: [&str; 4] = [
    "https://api-docs.deepseek.com/quick_start/pricing/",
    "https://api-docs.deepseek.com/api/create-chat-completion/",
    "https://openrouter.ai/docs/guides/routing/provider-selection",
    "https://openrouter.ai/api/v1/models/google/gemma-4-31b-it:free/endpoints",
];
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Safe public observations, never a provider bill or project-tier attestation.
#[derive(Serialize)]
pub struct ContractFacts {
    /// UTC observation time; freshness also uses the process monotonic clock.
    pub observed_unix_seconds: u64,
    /// Fixed page-order hashes; raw HTML/metadata never enters a receipt.
    pub sha256: Vec<String>,
    /// Four fixed GETs consumed; no hidden refresh or replacement is allowed.
    pub public_gets: u8,
    /// Fixed requested DeepSeek alias, with a separately documented version.
    pub documented_deepseek_version: &'static str,
    /// The sole admitted free route; not account/free-tier proof.
    pub openrouter_route: &'static str,
}
/// Successfully checked fixed facts. Private construction prevents a caller
/// supplying arbitrary fresh-price flags to activate the live factory.
pub struct FreshContracts {
    facts: ContractFacts,
    observed: Instant,
}
impl FreshContracts {
    /// Fetch only the four fixed public pages, once, with no Authorization,
    /// redirect, proxy or retry. Any missing/changed contract blocks key lookup.
    pub async fn acquire() -> Result<Self, Failure> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|_| Failure::NotReady)?;
        let observed = Instant::now();
        let mut hashes = Vec::new();
        for (index, url) in PAGES.iter().enumerate() {
            let body = tokio::time::timeout(Duration::from_secs(10), async {
                let mut response = client
                    .get(*url)
                    .send()
                    .await
                    .map_err(|_| Failure::NotReady)?;
                if response.status() != reqwest::StatusCode::OK
                    || response
                        .content_length()
                        .is_some_and(|size| size > MAX_BYTES as u64)
                {
                    return Err(Failure::NotReady);
                }
                let mut body = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(|_| Failure::NotReady)? {
                    if body
                        .len()
                        .checked_add(chunk.len())
                        .is_none_or(|size| size > MAX_BYTES)
                    {
                        return Err(Failure::NotReady);
                    }
                    body.extend_from_slice(&chunk);
                }
                Ok(body)
            })
            .await
            .map_err(|_| Failure::NotReady)??;
            validate_page(index, &body)?;
            hashes.push(native::sha256(&body)?);
        }
        Ok(Self {
            facts: ContractFacts {
                observed_unix_seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| Failure::NotReady)?
                    .as_secs(),
                sha256: hashes,
                public_gets: 4,
                documented_deepseek_version: "DeepSeek-V4.1-Flash-1017",
                openrouter_route: "google-ai-studio",
            },
            observed,
        })
    }
    /// Enforce the original15-minute observation window before each next slot.
    /// Expiry halts without a replacement call, key rotation or seventh GET.
    pub fn require_fresh(&self) -> Result<(), Failure> {
        if self.observed.elapsed() >= Duration::from_secs(15 * 60) {
            Err(Failure::NotReady)
        } else {
            Ok(())
        }
    }
    /// Immutable safe receipt data; no raw page or key becomes observable.
    pub fn facts(&self) -> &ContractFacts {
        &self.facts
    }
    /// Bind real source and code-owned policy/corpus bytes to checked evidence.
    pub fn identity(&self) -> Result<Identity, Failure> {
        self.require_fresh()?;
        Ok(Identity {
            source: native::source()?.into(),
            policy_sha256: native::sha256(include_bytes!("policy.rs"))?,
            corpus_sha256: native::sha256(
                format!(
                    "{}\n{}\n{}\n{}",
                    policy::DICTATION,
                    policy::REVIEW,
                    policy::CANCELLATION,
                    policy::ASK
                )
                .as_bytes(),
            )?,
            contract_sha256: self.facts.sha256.clone(),
        })
    }
}
fn zero(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            !text.is_empty()
                && text.bytes().all(|byte| matches!(byte, b'0' | b'.'))
                && text.bytes().filter(|b| *b == b'.').count() <= 1
                && text.bytes().any(|b| b == b'0')
        }
        Value::Number(value) => value.as_f64().is_some_and(|n| n.is_finite() && n == 0.0),
        _ => false,
    }
}
/// Validate one fixed public page through the production activation boundary.
/// Inputs are untrusted public bytes; errors disclose no rejected value. This
/// cannot manufacture a FreshContracts or change a destination/model policy.
pub fn validate_page(index: usize, body: &[u8]) -> Result<(), Failure> {
    if body.is_empty() || body.len() > MAX_BYTES {
        return Err(Failure::Contract);
    }
    let text = std::str::from_utf8(body).map_err(|_| Failure::Contract)?;
    let required: &[&str] = match index {
        0 => return validate_pricing(text),
        1 => &["thinking", "disabled", "max_tokens"],
        2 => &["allow_fallbacks", "require_parameters", "max_price", "only"],
        3 => return validate_endpoints(body),
        _ => return Err(Failure::Contract),
    };
    if required.iter().all(|part| text.contains(part)) {
        Ok(())
    } else {
        Err(Failure::Contract)
    }
}
fn pattern(expression: &str) -> Result<regex::Regex, Failure> {
    regex::Regex::new(expression).map_err(|_| Failure::Contract)
}
fn validate_pricing(html: &str) -> Result<(), Failure> {
    let tables = pattern(r"(?is)<table\b[^>]*>(.*?)</table>")?;
    let rows = pattern(r"(?is)<tr\b[^>]*>(.*?)</tr>")?;
    let cells = pattern(r"(?is)<(?:td|th)\b([^>]*)>(.*?)</(?:td|th)>")?;
    let spans = pattern(r#"(?i)(rowspan|colspan)\s*=\s*["']?([0-9]+)"#)?;
    let superscript = pattern(r"(?is)<sup\b[^>]*>.*?</sup>")?;
    let tags = pattern(r"(?s)<[^>]*>")?;
    let mut matched = 0;
    for table in tables.captures_iter(html) {
        let mut grid = vec![vec![None::<String>; 16]; 64];
        let mut row_count = 0;
        for (row_index, row) in rows.captures_iter(&table[1]).enumerate() {
            if row_index >= 64 {
                return Err(Failure::Contract);
            }
            row_count = row_index + 1;
            let mut column = 0;
            for cell in cells.captures_iter(&row[1]) {
                while column < 16 && grid[row_index][column].is_some() {
                    column += 1;
                }
                let (mut height, mut width) = (1, 1);
                for span in spans.captures_iter(&cell[1]) {
                    let value = span[2].parse::<usize>().map_err(|_| Failure::Contract)?;
                    if span[1].eq_ignore_ascii_case("rowspan") {
                        height = value;
                    } else {
                        width = value;
                    }
                }
                if height == 0 || width == 0 || row_index + height > 64 || column + width > 16 {
                    return Err(Failure::Contract);
                }
                let stripped = superscript.replace_all(&cell[2], "");
                let text = tags
                    .replace_all(&stripped, " ")
                    .replace("&nbsp;", " ")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                if text.len() > 1024 || text.contains('&') {
                    return Err(Failure::Contract);
                }
                for target in grid.iter_mut().skip(row_index).take(height) {
                    for value in target.iter_mut().skip(column).take(width) {
                        if value.is_some() {
                            return Err(Failure::Contract);
                        }
                        *value = Some(text.clone());
                    }
                }
                column += width;
            }
        }
        let rows = &grid[..row_count];
        let model_rows = rows
            .iter()
            .filter(|row| row[0].as_deref() == Some("MODEL"))
            .collect::<Vec<_>>();
        for header in model_rows {
            let positions = header
                .iter()
                .enumerate()
                .filter(|(_, value)| value.as_deref() == Some("deepseek-flash"))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if positions.is_empty() {
                continue;
            }
            if positions.len() != 1 {
                return Err(Failure::Contract);
            }
            let column = positions[0];
            let version = rows
                .iter()
                .filter(|row| row[0].as_deref() == Some("MODEL VERSION"))
                .collect::<Vec<_>>();
            if version.len() != 1
                || version[0][column].as_deref() != Some("DeepSeek-V4.1-Flash-1017")
            {
                return Err(Failure::Contract);
            }
            for (label, price) in [
                ("1M INPUT TOKENS (CACHE MISS)", "$0.3"),
                ("1M OUTPUT TOKENS", "$1.2"),
            ] {
                let prices = rows
                    .iter()
                    .filter(|row| {
                        row[0].as_deref() == Some("PRICING")
                            && row[1].as_deref() == Some(label)
                            && row[2].as_deref() == Some("PEAK")
                    })
                    .collect::<Vec<_>>();
                if prices.len() != 1
                    || prices[0][column]
                        .as_deref()
                        .is_none_or(|value| value.trim_end_matches('0') != price)
                {
                    return Err(Failure::Contract);
                }
            }
            matched += 1;
        }
    }
    if matched == 1 {
        Ok(())
    } else {
        Err(Failure::Contract)
    }
}
fn validate_endpoints(body: &[u8]) -> Result<(), Failure> {
    if !crate::workflow::experiment::no_numeric_underflow(body) {
        return Err(Failure::Contract);
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| Failure::Contract)?;
    let endpoints = value["data"]["endpoints"]
        .as_array()
        .filter(|items| !items.is_empty() && items.len() <= 128)
        .ok_or(Failure::Contract)?;
    let matches = endpoints
        .iter()
        .filter(|item| item["tag"].as_str() == Some("google-ai-studio"))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(Failure::Contract);
    }
    let endpoint = matches[0];
    if endpoint["status"].as_i64() != Some(0) {
        return Err(Failure::Contract);
    }
    let pricing = endpoint["pricing"].as_object().ok_or(Failure::Contract)?;
    if !["prompt", "completion"]
        .iter()
        .all(|field| pricing.get(*field).is_some_and(zero))
        || pricing.iter().any(|(name, price)| {
            !matches!(
                name.as_str(),
                "image" | "audio" | "input_audio" | "output_audio"
            ) && !zero(price)
        })
    {
        return Err(Failure::Contract);
    }
    let parameters = endpoint["supported_parameters"]
        .as_array()
        .ok_or(Failure::Contract)?;
    if !["reasoning", "max_tokens"].iter().all(|required| {
        parameters
            .iter()
            .any(|value| value.as_str() == Some(required))
    }) {
        return Err(Failure::Contract);
    }
    Ok(())
}
