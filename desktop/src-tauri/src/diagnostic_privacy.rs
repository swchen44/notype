//! Fixed-field privacy serializer. This is not a raw-log/settings ingestion API.
//! Sensitive-field masking is defensive only: final report fields remain the
//! original typed allowlist, which contains no credential fields.

/// Provenance supplied by the fixed renderer, never inferred from a regex.
#[allow(dead_code)] // Literal/unknown branches defend future formatting mistakes.
pub(crate) enum Encoding {
    /// An explicitly identified synthetic/literal opaque credential value.
    LiteralOpaque,
    /// Untrusted, encoded or otherwise ambiguous input; always fully masked.
    Unknown,
    /// Known metadata requiring its own allowlist validator.
    Metadata,
}

/// Sanitizes one bounded known field. Unknown fields are omitted; this helper
/// cannot add a field to the report renderer's independent fixed allowlist.
pub(crate) fn sanitize_field(name: &str, value: &str, encoding: Encoding) -> Option<String> {
    if name.len() > 64 || !name.is_ascii() {
        return None;
    }
    let name = name.to_ascii_lowercase().replace([' ', '_', '-'], "");
    if name == "version" {
        return Some(
            if !value.is_empty()
                && value.len() <= 32
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
                && value.bytes().any(|byte| byte.is_ascii_digit())
            {
                value
            } else {
                "unavailable"
            }
            .into(),
        );
    }
    if !matches!(
        name.as_str(),
        "key"
            | "apikey"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "idtoken"
            | "authtoken"
            | "bearertoken"
            | "githubtoken"
            | "clientsecret"
            | "secret"
            | "password"
            | "authorization"
            | "proxyauthorization"
            | "xapikey"
    ) {
        return None;
    }
    let full = || Some("********".into());
    if matches!(name.as_str(), "authorization" | "proxyauthorization")
        || !matches!(encoding, Encoding::LiteralOpaque)
    {
        return full();
    }
    let opaque = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-';
    let bytes = value.as_bytes();
    if value == "********"
        || (bytes.len() == 12
            && bytes[..2].iter().copied().all(opaque)
            && &bytes[2..10] == b"********"
            && bytes[10..].iter().copied().all(opaque))
    {
        return Some(value.into());
    }
    if bytes.len() < 12 || !bytes.iter().copied().all(opaque) {
        return full();
    }
    Some(format!(
        "{}********{}",
        &value[..2],
        &value[value.len() - 2..]
    ))
}
