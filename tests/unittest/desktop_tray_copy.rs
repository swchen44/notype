use super::last_transcript_text;
use notype_core::{SessionOptions, SessionService, TextSelection};

#[test]
fn tray_copy_uses_selected_result_or_saved_original() {
    let sessions = SessionService::in_memory().unwrap();
    let mut entry = sessions.create("raw", SessionOptions::default()).unwrap();
    entry.result = Some("processed".into());
    entry.selection = TextSelection::Polished;
    assert_eq!(last_transcript_text(&entry), "processed");
    entry.selection = TextSelection::Original;
    assert_eq!(last_transcript_text(&entry), "raw");
}

#[test]
fn tray_copy_falls_back_to_raw_when_processing_has_no_result() {
    let sessions = SessionService::in_memory().unwrap();
    let entry = sessions.create("raw", SessionOptions::default()).unwrap();
    assert_eq!(last_transcript_text(&entry), "raw");
}
