#[path="../../desktop/src-tauri/src/native_target_policy.rs"]
mod policy;
use policy::{control_kind,ControlKind};
#[test]
fn notepad_plus_plus_scintilla_is_a_distinct_supported_target() {
    assert_eq!(control_kind("Scintilla", "C:\\Program Files\\Notepad++\\notepad++.exe"),Some(ControlKind::Scintilla));
    assert_eq!(control_kind("Scintilla", "C:\\other.exe"),None);
    assert_eq!(control_kind("Chrome_RenderWidgetHostHWND", "C:\\chrome.exe"),None);
    assert_eq!(control_kind("Edit", "C:\\editor.exe"),Some(ControlKind::Edit));
}
