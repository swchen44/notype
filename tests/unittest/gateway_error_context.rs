use notype_core::gateway_error::GatewayError;
use serde_json::json;

#[test]
fn numeric_validation_can_refine_exact_context_metadata_without_overriding_other_statuses() {
    for outer_status in [400, 200] {
        let body =
            json!({"error":{"code":400,"metadata":{"error_type":"context_length_exceeded"}}});
        assert_eq!(
            GatewayError::classify(outer_status, Some(&body)),
            Some(GatewayError::ContextWindowExceeded),
            "outer HTTP {outer_status} with numeric validation and exact context metadata"
        );
    }
    for outer_status in [429, 200] {
        let body =
            json!({"error":{"code":429,"metadata":{"error_type":"context_length_exceeded"}}});
        assert_eq!(
            GatewayError::classify(outer_status, Some(&body)),
            Some(GatewayError::RateLimit)
        );
    }
    for outer_status in [400, 200] {
        let body = json!({"error":{"code":400,"metadata":{"error_type":"invalid_parameter"},"message":"context_length_exceeded"}});
        assert_eq!(
            GatewayError::classify(outer_status, Some(&body)),
            Some(GatewayError::UpstreamValidationError)
        );
    }
}
