//! Independent bounded CSV readback. The production encoder/export transaction
//! remains the sole writer and actual money remains UNKNOWN.
use super::journal::{Failure, Snapshot, State};
use notype_core::SessionEntry;
use serde::Serialize;
use std::collections::HashSet;

/// Safe independently reconciled export facts, without business text/model IDs.
#[derive(Serialize)]
pub struct CsvProof {
    /// Actual distinct core histories represented in the CSV.
    pub histories: usize,
    /// Actual durable possibly-sent calls, not a guessed slot count.
    pub observed_calls: u64,
    /// No actual currency/charge was invented from metadata or reservations.
    pub actual_fees_unknown: bool,
}
fn parse(bytes: &[u8]) -> Result<Vec<Vec<String>>, Failure> {
    let body = bytes
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .ok_or(Failure::Storage)?;
    if body.len() > 8 * 1024 * 1024 {
        return Err(Failure::Storage);
    }
    let mut index = 0;
    let mut rows = vec![];
    while index < body.len() {
        if rows.len() > 9 {
            return Err(Failure::Storage);
        }
        let mut row = vec![];
        loop {
            if body.get(index) != Some(&b'"') {
                return Err(Failure::Storage);
            }
            index += 1;
            let mut cell = vec![];
            loop {
                let byte = *body.get(index).ok_or(Failure::Storage)?;
                index += 1;
                if byte != b'"' {
                    cell.push(byte);
                    continue;
                }
                if body.get(index) == Some(&b'"') {
                    cell.push(b'"');
                    index += 1;
                    continue;
                }
                break;
            }
            row.push(String::from_utf8(cell).map_err(|_| Failure::Storage)?);
            if row.len() > 64 {
                return Err(Failure::Storage);
            }
            match body.get(index) {
                Some(b',') => {
                    index += 1;
                }
                Some(b'\r') if body.get(index + 1) == Some(&b'\n') => {
                    index += 2;
                    break;
                }
                _ => return Err(Failure::Storage),
            }
        }
        rows.push(row);
    }
    Ok(rows)
}
/// Parse actual BOM/quoted UTF-8 CSV independently of its writer and compare
/// it to public core entries and the durable authorization snapshot. Rejected
/// text is never formatted into an error/receipt/diagnostic event.
pub fn verify(
    bytes: &[u8],
    entries: &[SessionEntry],
    snapshot: &Snapshot,
) -> Result<CsvProof, Failure> {
    let rows = parse(bytes)?;
    let headers = rows.first().ok_or(Failure::Storage)?;
    if headers.len() != headers.iter().collect::<HashSet<_>>().len()
        || rows.len() != entries.len() + 1
        || entries.len() > 9
    {
        return Err(Failure::Storage);
    }
    let column = |name: &str| {
        headers
            .iter()
            .position(|value| value == name)
            .ok_or(Failure::Storage)
    };
    let get = |row: &[String], name: &str| -> Result<String, Failure> {
        row.get(column(name)?).cloned().ok_or(Failure::Storage)
    };
    let mut seen = HashSet::new();
    let mut calls = 0u64;
    for row in rows.iter().skip(1) {
        if row.len() != headers.len() {
            return Err(Failure::Storage);
        }
        let id = get(row, "history_id")?;
        if !seen.insert(id.clone()) {
            return Err(Failure::Storage);
        }
        let entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or(Failure::Storage)?;
        let expected_calls = entry.api_summary.observed_call_count;
        if get(row, "format_version")? != "2"
            || get(row, "original_text")? != entry.transcript
            || get(row, "result_text")? != entry.result.as_deref().unwrap_or("")
            || get(row, "back_translation")? != entry.back_translation.as_deref().unwrap_or("")
            || get(row, "api_observed_call_count")? != expected_calls.to_string()
            || get(row, "api_call_count")?
                != entry
                    .api_summary
                    .call_count
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            || get(row, "input_tokens_known_subtotal")?
                != entry
                    .api_summary
                    .input
                    .known_subtotal
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            || get(row, "output_tokens_known_subtotal")?
                != entry
                    .api_summary
                    .output
                    .known_subtotal
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            || get(row, "actual_fee_status")? != "unknown"
            || get(row, "actual_fee_reported_calls")? != "0"
            || get(row, "actual_fee_totals_by_currency")? != "[]"
            || get(row, "actual_fee_known_subtotals_by_currency")? != "[]"
            || !entry.api_summary.fees.totals.is_empty()
            || !entry.api_summary.fees.known_subtotals.is_empty()
            || !get(row, "recording_audio_seconds")?.is_empty()
            || !get(row, "asr_actual_model")?.is_empty()
            || !get(row, "asr_requested_model")?.is_empty()
        {
            return Err(Failure::Storage);
        }
        let options: serde_json::Value = serde_json::from_str(&get(row, "api_request_options")?)
            .map_err(|_| Failure::Storage)?;
        let expected = entry
            .api_summary
            .model_efforts
            .iter()
            .filter(|tuple| tuple.parameters.is_some())
            .collect::<Vec<_>>();
        if options != serde_json::to_value(expected).map_err(|_| Failure::Storage)? {
            return Err(Failure::Storage);
        }
        calls = calls.checked_add(expected_calls).ok_or(Failure::Storage)?;
    }
    let reserved = snapshot
        .records
        .iter()
        .filter(|record| {
            matches!(
                record.state,
                State::PossiblySent | State::Finished | State::Reconciled
            )
        })
        .count() as u64;
    if calls != reserved || calls > 12 {
        return Err(Failure::Storage);
    }
    Ok(CsvProof {
        histories: entries.len(),
        observed_calls: calls,
        actual_fees_unknown: true,
    })
}
