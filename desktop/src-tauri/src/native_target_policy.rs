//! Typed native control support; no handle, document text or title enters this policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind { Edit, Scintilla }
pub fn control_kind(class: &str, _image: &str) -> Option<ControlKind> {
    if ["Edit", "RichEdit20W", "RICHEDIT50W"].iter().any(|expected|class.eq_ignore_ascii_case(expected)) {Some(ControlKind::Edit)} else {None}
}
