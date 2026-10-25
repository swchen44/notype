//! Safe provider failure vocabulary shared by transport and history; no raw body is owned.
use std::fmt;

/// A provider failure containing only a fixed category. Credentials, URLs and
/// provider messages must never be attached to this Copy value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayError {
    /// The provider rejected authentication.
    AuthError,
}
impl GatewayError {
    /// Classify an authoritative HTTP status without retaining response content.
    pub fn from_status(status: u16) -> Option<Self> {
        (status == 401).then_some(Self::AuthError)
    }
    /// Fixed persisted history code for the forward or independent reverse call.
    pub fn history_code(self, back_translation: bool) -> &'static str {
        match (self, back_translation) {
            (Self::AuthError, false) => "provider-auth",
            (Self::AuthError, true) => "back-provider-auth",
        }
    }
}
impl fmt::Display for GatewayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Provider authentication failed.")
    }
}
impl std::error::Error for GatewayError {}
