#[path = "../../desktop/src-tauri/src/diagnostic_privacy.rs"]
mod privacy;
use privacy::{sanitize_field, Encoding};

#[test]
fn recognized_literal_secrets_use_exact_cutoff_and_fixed_masks_idempotently() {
    for name in [
        "key",
        "API_Key",
        "api-key",
        "ToKeN",
        "access_token",
        "refresh-token",
        "id token",
        "auth_token",
        "bearer_token",
        "github_token",
        "client_secret",
        "secret",
        "password",
        "X-API-Key",
    ] {
        for value in ["", "a", "abcdefghijk"] {
            assert_eq!(
                sanitize_field(name, value, Encoding::LiteralOpaque).as_deref(),
                Some("********")
            );
        }
        let masked = sanitize_field(name, "abcdefghijkl", Encoding::LiteralOpaque).unwrap();
        assert_eq!(masked, "ab********kl");
        assert_eq!(
            sanitize_field(name, &masked, Encoding::LiteralOpaque).unwrap(),
            masked
        );
        assert_eq!(
            sanitize_field(name, "abcdefghijklmnop", Encoding::LiteralOpaque).unwrap(),
            "ab********op"
        );
    }
}

#[test]
fn ambiguous_encoded_authorization_and_unknown_fields_fail_closed() {
    for value in [
        "Bearer abcdefghijkl",
        "bEaReR abcdefghijkl",
        "header.payload.signature",
        "?token=abcdefghijkl&key=another",
        "%61bcdefghijkl",
        "base64:YWJjZGVmZ2hpamts",
        "\\u0061bcdefghijkl",
        "原稿秘密秘密秘密",
        "abc\nsecret",
        "abc***private",
    ] {
        assert_eq!(
            sanitize_field("api_key", value, Encoding::LiteralOpaque).unwrap(),
            "********"
        );
    }
    for name in ["Authorization", "Proxy-Authorization"] {
        assert_eq!(
            sanitize_field(name, "abcdefghijkl", Encoding::LiteralOpaque).unwrap(),
            "********"
        );
    }
    assert_eq!(
        sanitize_field("token", "abcdefghijkl", Encoding::Unknown).unwrap(),
        "********"
    );
    assert_eq!(
        sanitize_field("unknown-secret", "abcdefghijkl", Encoding::LiteralOpaque),
        None
    );
    assert_eq!(
        sanitize_field(&"a".repeat(65), "secret", Encoding::Unknown),
        None
    );
    assert_eq!(
        sanitize_field(
            "version",
            "Bearer abcdefghijkl\napi_key=second-secret",
            Encoding::Metadata
        )
        .unwrap(),
        "unavailable"
    );
}
