//! Typed credential-free history projection shared by native IPC and its portable
//! contract tests. No app handle/settings/diagnostics can enter this module.
use notype_core::observations as core;
use serde::Serialize;
use specta::Type;

/// Actual received audio duration and observed completeness.
#[derive(Serialize, Type)]
pub struct HistoryRecordingDto {
    /// Milliseconds before VAD/resampling/decoder padding, not UI display time.
    pub audio_ms: u64,
    /// complete, incomplete or unknown capture quality.
    pub quality: String,
}
/// Identity bound to the engine that produced the original.
#[derive(Serialize, Type)]
pub struct HistoryModelDto {
    /// Stable producer/catalog ID.
    pub id: String,
    /// Producer display name when known.
    pub name: Option<String>,
    /// Backend label when known.
    pub backend: Option<String>,
}
/// Durable basic observations for a history record; None means unobserved.
#[derive(Serialize, Type)]
pub struct HistoryObservationsDto {
    /// Actual received audio summary, independent of diagnostics.
    pub recording: Option<HistoryRecordingDto>,
    /// ASR selection captured for this recording.
    pub requested_asr_model: Option<String>,
    /// Actual successful producer, including a fallback producer.
    pub actual_asr_model: Option<HistoryModelDto>,
    /// Original's mixed count under count_rule.
    pub original_count: Option<u64>,
    /// Current result's mixed count; absent when no result exists.
    pub result_count: Option<u64>,
    /// Versioned public counting rule, currently zh-en-v1.
    pub count_rule: String,
    /// Accepted stop to durably saved normalized original.
    pub asr_after_stop_ms: Option<u64>,
    /// Initial stop to usable result, excluding human review/delivery.
    pub initial_stop_to_result_ms: Option<u64>,
    /// initial or retry, with no human waiting in the measured interval.
    pub latest_processing_kind: Option<String>,
    /// Latest observed processing interval.
    pub latest_processing_ms: Option<u64>,
    /// Latest API stage, None when no actual network call occurred.
    pub latest_api_ms: Option<u64>,
    /// Application outcome label, not a raw provider error.
    pub latest_outcome: Option<String>,
    /// Accounting cannot be asserted complete for every call.
    pub api_metadata_unavailable: bool,
}
impl From<core::SessionObservations> for HistoryObservationsDto {
    fn from(value: core::SessionObservations) -> Self {
        Self {
            recording: value.recording.map(|r| HistoryRecordingDto {
                audio_ms: r.audio_ms(),
                quality: r.quality().into(),
            }),
            requested_asr_model: value.requested_asr_model,
            actual_asr_model: value.actual_asr_model.map(|m| HistoryModelDto {
                id: m.id,
                name: m.name,
                backend: m.backend,
            }),
            original_count: value.original_count,
            result_count: value.result_count,
            count_rule: value.count_rule,
            asr_after_stop_ms: value.asr_after_stop_ms,
            initial_stop_to_result_ms: value.initial_stop_to_result_ms,
            latest_processing_kind: value.latest_processing_kind,
            latest_processing_ms: value.latest_processing_ms,
            latest_api_ms: value.latest_api_ms,
            latest_outcome: value.latest_outcome,
            api_metadata_unavailable: value.api_metadata_unavailable,
        }
    }
}
/// Completeness is independent for input, output and actual money.
#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HistoryMetricStatus {
    /// No actual API use and no instrumentation gap.
    NotApplicable,
    /// Every actual call reported the relevant field.
    Complete,
    /// A known subtotal, not a complete history total.
    Partial,
    /// Missing or invalid data, never a fabricated zero.
    Unknown,
}
impl From<core::Completeness> for HistoryMetricStatus {
    fn from(value: core::Completeness) -> Self {
        match value {
            core::Completeness::NotApplicable => Self::NotApplicable,
            core::Completeness::Complete => Self::Complete,
            core::Completeness::Partial => Self::Partial,
            core::Completeness::Unknown => Self::Unknown,
        }
    }
}
/// One independent nullable token total.
#[derive(Serialize, Type)]
pub struct HistoryUsageDto {
    /// Completeness of this field.
    pub status: HistoryMetricStatus,
    /// Full total only when complete.
    pub total: Option<u64>,
    /// Sum of known observations, explicitly possibly incomplete.
    pub known_subtotal: Option<u64>,
    /// Number of actual calls reporting this field.
    pub reported_calls: u64,
}
impl From<core::UsageTotal> for HistoryUsageDto {
    fn from(value: core::UsageTotal) -> Self {
        Self {
            status: value.status.into(),
            total: value.total,
            known_subtotal: value.known_subtotal,
            reported_calls: value.reported_calls,
        }
    }
}
/// Exact actual money in one explicit currency.
#[derive(Serialize, Type)]
pub struct HistoryCurrencyAmountDto {
    /// Explicit currency; no FX conversion occurs.
    pub currency: String,
    /// Exact decimal string, never floating-point or a token-price estimate.
    pub amount: String,
}
impl From<core::CurrencyAmount> for HistoryCurrencyAmountDto {
    fn from(value: core::CurrencyAmount) -> Self {
        Self {
            currency: value.currency,
            amount: value.amount,
        }
    }
}
/// Actual returned fees only, separated from partial known subtotals.
#[derive(Serialize, Type)]
pub struct HistoryFeeDto {
    /// Completeness across actual calls.
    pub status: HistoryMetricStatus,
    /// Complete totals by currency; empty when not complete.
    pub totals: Vec<HistoryCurrencyAmountDto>,
    /// Known observed amounts, not necessarily the full fee.
    pub known_subtotals: Vec<HistoryCurrencyAmountDto>,
    /// Number of actual calls returning amount and currency.
    pub reported_calls: u64,
}
/// Preserve requested/reported model and effort associations.
#[derive(Serialize, Type)]
pub struct HistoryModelEffortDto {
    /// Requested model name, if known.
    pub requested_model: Option<String>,
    /// Explicit response model name, if returned.
    pub reported_model: Option<String>,
    /// Explicit request effort; absent means provider-default omission.
    pub requested_effort: Option<String>,
    /// Explicit response effort, never inferred from the request.
    pub reported_effort: Option<String>,
}
/// All-call summary for one history ID, including retries/back-translation.
#[derive(Serialize, Type)]
pub struct HistoryApiSummaryDto {
    /// Full actual-call count only when instrumentation is complete.
    pub call_count: Option<u64>,
    /// Actual possibly-sent calls observed in the ledger.
    pub observed_call_count: u64,
    /// Independent prompt/input usage.
    pub input: HistoryUsageDto,
    /// Independent completion/output usage.
    pub output: HistoryUsageDto,
    /// Actual reported money, with no estimates or currency conversion.
    pub fees: HistoryFeeDto,
    /// Sorted distinct requested models.
    pub requested_models: Vec<String>,
    /// Sorted distinct reported models.
    pub reported_models: Vec<String>,
    /// Sorted distinct model/effort associations.
    pub model_efforts: Vec<HistoryModelEffortDto>,
}
impl From<core::ApiSummary> for HistoryApiSummaryDto {
    fn from(value: core::ApiSummary) -> Self {
        Self {
            call_count: value.call_count,
            observed_call_count: value.observed_call_count,
            input: value.input.into(),
            output: value.output.into(),
            fees: HistoryFeeDto {
                status: value.fees.status.into(),
                totals: value.fees.totals.into_iter().map(Into::into).collect(),
                known_subtotals: value
                    .fees
                    .known_subtotals
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                reported_calls: value.fees.reported_calls,
            },
            requested_models: value.requested_models,
            reported_models: value.reported_models,
            model_efforts: value
                .model_efforts
                .into_iter()
                .map(|m| HistoryModelEffortDto {
                    requested_model: m.requested_model,
                    reported_model: m.reported_model,
                    requested_effort: m.requested_effort,
                    reported_effort: m.reported_effort,
                })
                .collect(),
        }
    }
}
/// Outcome of the explicit save action; paths are never returned in IPC/errors.
#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HistoryCsvExportStatus {
    /// Complete snapshot file atomically committed.
    Saved,
    /// User cancelled before a destination was committed.
    Cancelled,
}
/// Native export acknowledgement, separate from history processing status.
#[derive(Serialize, Type)]
pub struct HistoryCsvExportResult {
    /// Explicit saved/cancelled result.
    pub status: HistoryCsvExportStatus,
    /// Exported history IDs when saved, otherwise absent.
    pub rows: Option<u64>,
}
