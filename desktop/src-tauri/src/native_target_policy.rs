//! Typed native control support; no handle, document text or title enters this policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKind {
    Edit,
    Scintilla,
}
pub fn control_kind(class: &str, image: &str) -> Option<ControlKind> {
    if ["Edit", "RichEdit20W", "RICHEDIT50W"]
        .iter()
        .any(|expected| class.eq_ignore_ascii_case(expected))
    {
        return Some(ControlKind::Edit);
    }
    let image = image.rsplit(['\\', '/']).next().unwrap_or("");
    if class.eq_ignore_ascii_case("Scintilla")
        && (image.eq_ignore_ascii_case("notepad++") || image.eq_ignore_ascii_case("notepad++.exe"))
    {
        Some(ControlKind::Scintilla)
    } else {
        None
    }
}

/// Scintilla positions are encoded bytes. Avoid lossy insertion in legacy encodings.
pub fn scintilla_text(
    text: &str,
    code_page: usize,
    eol: usize,
    convert_endings: bool,
) -> Option<String> {
    if code_page != 65001 && !text.is_ascii() {
        return None;
    }
    let ending = match eol {
        0 => "\r\n",
        1 => "\r",
        2 => "\n",
        _ => return None,
    };
    if convert_endings {
        Some(
            text.replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', ending),
        )
    } else {
        Some(text.to_owned())
    }
}
