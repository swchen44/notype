use notype_core::gateway_error::GatewayError;

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
