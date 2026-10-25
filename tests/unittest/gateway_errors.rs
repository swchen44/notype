use notype_core::gateway_error::GatewayError;
use serde_json::json;

#[test]
fn authentication_failure_has_safe_forward_and_reverse_history_codes() {
    let error = GatewayError::from_status(401).unwrap();
    assert_eq!(error, GatewayError::AuthError);
    assert_eq!(error.history_code(false), "provider-auth");
    assert_eq!(error.history_code(true), "back-provider-auth");
    assert_eq!(error.to_string(), "Provider authentication failed.");
    assert_eq!(format!("{error:?}"), "AuthError");
}

#[test]
fn success_status_is_not_a_provider_error() {
    assert_eq!(GatewayError::from_status(200), None);
}

#[test]
fn authoritative_status_and_exact_structural_paths_ignore_conflicting_messages() {
    let conflict = json!({"error":{"code":"context_length_exceeded","message":"Bearer secret"}});
    for (status, expected) in [
        (401, GatewayError::AuthError),
        (402, GatewayError::InsufficientBalance),
        (403, GatewayError::PermissionDenied),
        (429, GatewayError::RateLimit),
        (408, GatewayError::ProviderTimeout),
        (504, GatewayError::ProviderTimeout),
        (500, GatewayError::UpstreamUnavailable),
        (502, GatewayError::UpstreamUnavailable),
        (503, GatewayError::UpstreamUnavailable),
        (599, GatewayError::UpstreamUnavailable),
        (302, GatewayError::Protocol),
        (418, GatewayError::Unknown { status: 418 }),
    ] {
        assert_eq!(
            GatewayError::classify(status, Some(&conflict)),
            Some(expected)
        );
        assert_eq!(GatewayError::classify(status, None), Some(expected));
    }
    for status in [400, 404, 413, 422] {
        assert_eq!(
            GatewayError::classify(status, Some(&conflict)),
            Some(GatewayError::ContextWindowExceeded)
        );
        assert_eq!(
            GatewayError::classify(
                status,
                Some(&json!({"error":{"message":"context_length_exceeded"}}))
            ),
            Some(GatewayError::UpstreamValidationError)
        );
    }
    for error in [
        json!({"code":"context_length_exceeded"}),
        json!({"type":"context_window_exceeded"}),
        json!({"metadata":{"error_type":"context_length_exceeded"}}),
    ] {
        assert_eq!(
            GatewayError::classify(200, Some(&json!({"error":error}))),
            Some(GatewayError::ContextWindowExceeded)
        );
    }
    assert_eq!(
        GatewayError::classify(
            200,
            Some(
                &json!({"error":{"status":401,"code":429,"metadata":{"error_type":"context_length_exceeded"}}})
            )
        ),
        Some(GatewayError::AuthError)
    );
    assert_eq!(
        GatewayError::classify(
            200,
            Some(&json!({"error":{"code":429,"type":"context_length_exceeded"}}))
        ),
        Some(GatewayError::RateLimit)
    );
}

#[test]
fn every_allowed_code_path_maps_without_substring_inference() {
    for (code, expected) in [
        ("invalid_api_key", GatewayError::AuthError),
        ("authentication_error", GatewayError::AuthError),
        ("permission_denied", GatewayError::PermissionDenied),
        ("insufficient_permissions", GatewayError::PermissionDenied),
        ("insufficient_quota", GatewayError::InsufficientBalance),
        ("insufficient_balance", GatewayError::InsufficientBalance),
        ("rate_limit_exceeded", GatewayError::RateLimit),
        ("rate_limit_error", GatewayError::RateLimit),
        ("request_timeout", GatewayError::ProviderTimeout),
        ("timeout", GatewayError::ProviderTimeout),
        ("server_error", GatewayError::UpstreamUnavailable),
        ("service_unavailable", GatewayError::UpstreamUnavailable),
        (
            "invalid_request_error",
            GatewayError::UpstreamValidationError,
        ),
        ("invalid_parameter", GatewayError::UpstreamValidationError),
    ] {
        for error in [
            json!({"code":code}),
            json!({"type":code}),
            json!({"metadata":{"error_type":code}}),
        ] {
            assert_eq!(
                GatewayError::classify(200, Some(&json!({"error":error}))),
                Some(expected)
            );
            assert_eq!(
                GatewayError::classify(200, Some(&json!({"choices":[{"error":error}]}))),
                Some(expected)
            );
        }
    }
    for error in [
        json!(false),
        json!({"code":"401"}),
        json!({"message":"rate_limit_exceeded"}),
        json!({"metadata":{"error_type":"context_length_exceeded:secret"}}),
    ] {
        assert_eq!(
            GatewayError::classify(200, Some(&json!({"error":error}))),
            Some(GatewayError::Protocol)
        );
    }
    assert_eq!(
        GatewayError::classify(200, Some(&json!({"error":null,"choices":[{"error":null}]}))),
        None
    );
    assert_eq!(
        GatewayError::classify(
            200,
            Some(&json!({"choices":[{"message":{"content":"ok"}},{"error":{"code":402}}]}))
        ),
        Some(GatewayError::InsufficientBalance)
    );
}

#[test]
fn safe_formatting_and_envelopes_never_own_untrusted_provider_data() {
    let malicious = json!({"error":{"code":"unknown-private-code","message":"Bearer KEY_SENTINEL https://private.test/?token=secret C:\\Users\\private RAW_SPEECH","metadata":{"raw":"RAW_JSON_SENTINEL"}}});
    for status in [200, 401, 418, 503] {
        let error = GatewayError::classify(status, Some(&malicious)).unwrap();
        let rendered = format!(
            "{error:?} {error} {} {} {}",
            error.openai_envelope(),
            error.history_code(false),
            error.history_code(true)
        );
        for secret in [
            "KEY_SENTINEL",
            "private",
            "RAW_SPEECH",
            "RAW_JSON_SENTINEL",
            "unknown-private-code",
            "Bearer",
            "token=",
        ] {
            assert!(
                !rendered.contains(secret),
                "unexpected upstream data: {rendered}"
            );
        }
        assert_eq!(error.openai_envelope()["error"]["message"], error.message());
    }
    assert_eq!(GatewayError::Unknown { status: 418 }.http_status(), 418);
    assert_eq!(GatewayError::Unknown { status: 200 }.http_status(), 502);
    assert_eq!(GatewayError::ProviderTimeout.http_status(), 504);
    assert_eq!(
        GatewayError::AuthError.openai_envelope(),
        json!({"error":{"message":"Provider authentication failed.","type":"provider-auth","code":"provider-auth"}})
    );
}
