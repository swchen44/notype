#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/commands/history_metrics.rs"]
mod history_metrics;
use history_metrics::HistoryObservationsDto;
use notype_core::observations::{ModelIdentity, RecordingObservation, SessionObservations};

#[test]
fn native_metrics_contract_exposes_received_milliseconds_and_unknowns_without_settings() {
    let observations = SessionObservations {
        recording: Some(RecordingObservation::new(24_000, 48_000, Some(0)).unwrap()),
        original_count: Some(6),
        requested_asr_model: Some("selected-model".into()),
        latest_processing_error: Some("provider-auth".into()),
        actual_asr_model: Some(ModelIdentity {
            id: "producer-model".into(),
            name: None,
            backend: None,
        }),
        ..Default::default()
    };
    let value = serde_json::to_value(HistoryObservationsDto::from(observations)).unwrap();
    assert_eq!(value["recording"]["audio_ms"], 500);
    assert_eq!(value["recording"]["quality"], "complete");
    assert_eq!(value["original_count"], 6);
    assert!(value["result_count"].is_null());
    assert_eq!(value["requested_asr_model"], "selected-model");
    assert_eq!(value["actual_asr_model"]["id"], "producer-model");
    assert_eq!(value["latest_processing_error"], "provider-auth");
    assert!(value.get("api_key").is_none());
    assert!(value.get("endpoint").is_none());
}
