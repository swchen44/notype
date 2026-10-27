#[path = "../../desktop/src-tauri/src/native_target_policy.rs"]
mod policy;
use policy::{control_kind, ControlKind};
#[test]
fn notepad_plus_plus_scintilla_is_a_distinct_supported_target() {
    assert_eq!(
        control_kind("Scintilla", "C:\\Program Files\\Notepad++\\notepad++.exe"),
        Some(ControlKind::Scintilla)
    );
    assert_eq!(control_kind("Scintilla", "C:\\other.exe"), None);
    assert_eq!(
        control_kind("Chrome_RenderWidgetHostHWND", "C:\\chrome.exe"),
        None
    );
    assert_eq!(
        control_kind("Edit", "C:\\editor.exe"),
        Some(ControlKind::Edit)
    );
}

#[test]
fn utf8_multiline_paste_obeys_document_endings_without_loss() {
    assert_eq!(
        policy::scintilla_text("中文 🧪\nOpenAI\r\nend\r", 65001, 0, true).as_deref(),
        Some("中文 🧪\r\nOpenAI\r\nend\r\n")
    );
    assert_eq!(
        policy::scintilla_text("a\r\nb\rc\n", 65001, 2, true).as_deref(),
        Some("a\nb\nc\n")
    );
    assert_eq!(
        policy::scintilla_text("a\r\nb", 65001, 2, false).as_deref(),
        Some("a\r\nb")
    );
    assert!(policy::scintilla_text("中文", 1252, 0, true).is_none());
    assert_eq!(
        policy::scintilla_text("ASCII", 1252, 0, true).as_deref(),
        Some("ASCII")
    );
    assert!(policy::scintilla_text("ASCII", 65001, 3, true).is_none());
}
