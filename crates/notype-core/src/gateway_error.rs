//! Safe provider failure vocabulary shared by transport and history; no raw body is owned.
use serde_json::{json, Value};
use std::fmt;

/// A provider failure containing only a fixed category. Credentials, URLs and
/// provider messages must never be attached to this Copy value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayError {
    /// The provider rejected authentication.
    AuthError,
    /// The account cannot use the selected resource.
    PermissionDenied,
    /// The service rejected the account's balance or quota.
    InsufficientBalance,
    /// The provider limited request frequency; this does not authorize a retry.
    RateLimit,
    /// An exact structural provider code identified the model's context limit.
    ContextWindowExceeded,
    /// The provider or transport reported a deadline expiry; billing is unknown.
    ProviderTimeout,
    /// The service rejected the endpoint, model or request parameters.
    UpstreamValidationError,
    /// The upstream service is unavailable.
    UpstreamUnavailable,
    /// Connecting or receiving the response failed without a timeout.
    Transport,
    /// Redirects and unrecognized in-band error envelopes are not valid results.
    Protocol,
    /// The response exceeded the bounded parser's byte limit.
    Oversize,
    /// An unrecognized HTTP failure; only the numeric status is retained.
    Unknown {
        /// Observed HTTP status, never a provider message or metadata object.
        status: u16,
    },
}
impl GatewayError {
    /// Classify an authoritative HTTP status without retaining response content.
    pub fn from_status(status: u16) -> Option<Self> {
        Some(match status {
            200..=299 => return None,
            401 => Self::AuthError,
            403 => Self::PermissionDenied,
            402 => Self::InsufficientBalance,
            429 => Self::RateLimit,
            408 | 504 => Self::ProviderTimeout,
            500..=599 => Self::UpstreamUnavailable,
            400 | 404 | 413 | 422 => Self::UpstreamValidationError,
            300..=399 => Self::Protocol,
            _ => Self::Unknown { status },
        })
    }
    /// Classify a bounded, already parsed response without owning any JSON data.
    /// HTTP failure status wins over conflicting JSON. Validation statuses may
    /// refine to context-limit only from an exact structural code. For 2xx,
    /// non-null top-level/choice errors and error finish reasons precede text.
    pub fn classify(status: u16, body: Option<&Value>) -> Option<Self> {
        if let Some(error) = Self::from_status(status) {
            return Some(
                if error == Self::UpstreamValidationError
                    && body.and_then(|body| structural_error(&body["error"]))
                        == Some(Self::ContextWindowExceeded)
                {
                    Self::ContextWindowExceeded
                } else {
                    error
                },
            );
        }
        let body = body?;
        if let Some(error) = body.get("error").filter(|error| !error.is_null()) {
            return Some(structural_error(error).unwrap_or(Self::Protocol));
        }
        for choice in body["choices"].as_array().into_iter().flatten() {
            if let Some(error) = choice.get("error").filter(|error| !error.is_null()) {
                return Some(structural_error(error).unwrap_or(Self::Protocol));
            }
            if choice["finish_reason"] == "error" {
                return Some(Self::Protocol);
            }
        }
        None
    }
    /// Fixed persisted history code for the forward or independent reverse call.
    pub fn history_code(self, back_translation: bool) -> &'static str {
        match (self, back_translation) {
            (Self::AuthError, false) => "provider-auth",
            (Self::AuthError, true) => "back-provider-auth",
            (Self::PermissionDenied, false) => "provider-permission",
            (Self::PermissionDenied, true) => "back-provider-permission",
            (Self::InsufficientBalance, false) => "provider-balance",
            (Self::InsufficientBalance, true) => "back-provider-balance",
            (Self::RateLimit, false) => "provider-rate-limit",
            (Self::RateLimit, true) => "back-provider-rate-limit",
            (Self::ContextWindowExceeded, false) => "provider-context",
            (Self::ContextWindowExceeded, true) => "back-provider-context",
            (Self::ProviderTimeout, false) => "provider-timeout",
            (Self::ProviderTimeout, true) => "back-provider-timeout",
            (Self::UpstreamValidationError, false) => "provider-validation",
            (Self::UpstreamValidationError, true) => "back-provider-validation",
            (Self::UpstreamUnavailable, false) => "provider-unavailable",
            (Self::UpstreamUnavailable, true) => "back-provider-unavailable",
            (Self::Transport, false) => "provider-transport",
            (Self::Transport, true) => "back-provider-transport",
            (Self::Protocol, false) => "provider-protocol",
            (Self::Protocol, true) => "back-provider-protocol",
            (Self::Oversize, false) => "provider-oversize",
            (Self::Oversize, true) => "back-provider-oversize",
            (Self::Unknown { .. }, false) => "provider-unknown",
            (Self::Unknown { .. }, true) => "back-provider-unknown",
        }
    }
    /// Safe constant English message for non-UI callers. UI uses localized codes.
    pub fn message(self) -> &'static str {
        match self {
            Self::AuthError => "Provider authentication failed.",
            Self::PermissionDenied => "Provider permission denied.",
            Self::InsufficientBalance => "Provider balance is insufficient.",
            Self::RateLimit => "Provider rate limit exceeded.",
            Self::ContextWindowExceeded => "Provider context limit exceeded.",
            Self::ProviderTimeout => "Provider request timed out; billing may have occurred.",
            Self::UpstreamValidationError => "Provider rejected the request parameters.",
            Self::UpstreamUnavailable => "Provider is unavailable.",
            Self::Transport => "Provider connection or response transfer failed.",
            Self::Protocol => "Provider returned an invalid response.",
            Self::Oversize => "Provider response exceeded the size limit.",
            Self::Unknown { .. } => "Provider returned an unrecognized error.",
        }
    }
    /// Canonical status for a future HTTP adapter, with no listener or retry policy.
    pub fn http_status(self) -> u16 {
        match self {
            Self::AuthError => 401,
            Self::PermissionDenied => 403,
            Self::InsufficientBalance => 402,
            Self::RateLimit => 429,
            Self::ContextWindowExceeded | Self::UpstreamValidationError => 400,
            Self::ProviderTimeout => 504,
            Self::UpstreamUnavailable => 503,
            Self::Unknown { status } if (400..=599).contains(&status) => status,
            _ => 502,
        }
    }
    /// OpenAI-shaped error made exclusively from constants, never upstream JSON.
    /// This pure projection neither serializes provider bodies nor serves HTTP.
    pub fn openai_envelope(self) -> Value {
        json!({"error":{"message":self.message(),"type":self.history_code(false),"code":self.history_code(false)}})
    }
}
impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
impl std::error::Error for GatewayError {}

// Structural paths are intentionally narrow: error.status, numeric error.code,
// then exact string error.code/error.type/error.metadata.error_type. The caller
// selects a top-level or choice error object; natural-language messages are ignored.
fn structural_error(error: &Value) -> Option<GatewayError> {
    for value in [&error["status"], &error["code"]] {
        if let Some(status) = value.as_u64().filter(|status| (400..=599).contains(status)) {
            return GatewayError::from_status(status as u16);
        }
    }
    for value in [
        &error["code"],
        &error["type"],
        &error["metadata"]["error_type"],
    ] {
        let category = match value.as_str() {
            Some("context_length_exceeded" | "context_window_exceeded") => {
                GatewayError::ContextWindowExceeded
            }
            Some("invalid_api_key" | "authentication_error") => GatewayError::AuthError,
            Some("permission_denied" | "insufficient_permissions") => {
                GatewayError::PermissionDenied
            }
            Some("insufficient_quota" | "insufficient_balance") => {
                GatewayError::InsufficientBalance
            }
            Some("rate_limit_exceeded" | "rate_limit_error") => GatewayError::RateLimit,
            Some("request_timeout" | "timeout") => GatewayError::ProviderTimeout,
            Some("server_error" | "service_unavailable") => GatewayError::UpstreamUnavailable,
            Some("invalid_request_error" | "invalid_parameter") => {
                GatewayError::UpstreamValidationError
            }
            _ => continue,
        };
        return Some(category);
    }
    None
}
