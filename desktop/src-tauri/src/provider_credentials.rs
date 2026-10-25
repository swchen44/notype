//! Selected-connection credential policy. No process-env/repository .env lookup.
use super::{Authentication, ConfigError, ConnectionDraft, ConnectionKind, ProviderDialect};
use std::{fs::File, io::Read, path::PathBuf};
/// Exactly the three user-approved public-provider names; no arbitrary variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicKey {
    /// DEEPSEEK_API_KEY at the exact documented DeepSeek route.
    DeepSeek,
    /// OPENROUTER_API_KEY at the exact documented OpenRouter route.
    OpenRouter,
    /// GEMINI_API_KEY at the exact documented Gemini compatibility route.
    Gemini,
}
impl PublicKey {
    /// Fixed allowlisted variable name, not its value.
    pub fn name(self) -> &'static str {
        match self {
            Self::DeepSeek => "DEEPSEEK_API_KEY",
            Self::OpenRouter => "OPENROUTER_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
        }
    }
}
/// Owned secret-file boundary. Only selected eligible fallback calls this port.
pub trait CredentialReader: Send + Sync {
    /// Resolve one allowlisted variable without exporting other values.
    fn read(&self, key: PublicKey) -> Result<Option<String>, ConfigError>;
}
/// Exact preset eligibility; custom/local/anonymous never receive public keys.
pub fn public_key(draft: &ConnectionDraft) -> Option<PublicKey> {
    if draft.kind != ConnectionKind::Provider || draft.authentication != Authentication::Bearer {
        return None;
    }
    let endpoint = super::canonical_chat(draft).ok()?;
    match (draft.dialect, endpoint.as_str()) {
        (ProviderDialect::Deepseek, "https://api.deepseek.com/chat/completions") => {
            Some(PublicKey::DeepSeek)
        }
        (ProviderDialect::Openrouter, "https://openrouter.ai/api/v1/chat/completions") => {
            Some(PublicKey::OpenRouter)
        }
        (
            ProviderDialect::Gemini,
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
        ) => Some(PublicKey::Gemini),
        _ => None,
    }
}
/// Parse a bounded literal dotenv subset: optional `export`, exact NAME=value,
/// optional single/double quotes and trailing comments. No escapes, expansion,
/// command execution or process environment mutation. Other values are ignored.
pub fn parse(bytes: &[u8], selected: PublicKey) -> Result<Option<String>, ConfigError> {
    if bytes.len() > 65_536 {
        return Err(ConfigError("configuration-home-file-invalid"));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ConfigError("configuration-home-file-invalid"))?
        .trim_start_matches('\u{feff}');
    let mut seen = std::collections::HashSet::new();
    let mut selected_value = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((name, value)) = line.split_once('=') else {
            if [
                PublicKey::DeepSeek,
                PublicKey::OpenRouter,
                PublicKey::Gemini,
            ]
            .iter()
            .any(|key| line.starts_with(key.name()))
            {
                return Err(ConfigError("configuration-home-file-invalid"));
            }
            continue;
        };
        let name = name.trim();
        if ![
            PublicKey::DeepSeek,
            PublicKey::OpenRouter,
            PublicKey::Gemini,
        ]
        .iter()
        .any(|key| name == key.name())
        {
            continue;
        }
        if !seen.insert(name) {
            return Err(ConfigError("configuration-home-file-invalid"));
        }
        if name != selected.name() {
            continue;
        }
        let value = value.trim();
        let value = if let Some(quote) = value
            .chars()
            .next()
            .filter(|quote| matches!(quote, '\'' | '"'))
        {
            let rest = &value[1..];
            let end = rest
                .find(quote)
                .ok_or(ConfigError("configuration-home-file-invalid"))?;
            let tail = rest[end + 1..].trim();
            if !tail.is_empty() && !tail.starts_with('#') {
                return Err(ConfigError("configuration-home-file-invalid"));
            }
            &rest[..end]
        } else {
            value
                .split_once(" #")
                .map_or(value, |(value, _)| value)
                .trim_end()
        };
        if value.len() > 8192 || value.chars().any(char::is_control) {
            return Err(ConfigError("configuration-home-file-invalid"));
        }
        if !value.is_empty() {
            selected_value = Some(value.to_owned());
        }
    }
    Ok(selected_value)
}
/// A fixed OS-home file adapter, injectable with an owned temporary home in tests.
/// Construction reads nothing; the home path is never returned by settings DTOs.
pub struct HomeFileReader {
    home: PathBuf,
}
impl HomeFileReader {
    /// Bind an already resolved OS home, or an explicitly owned test home.
    pub fn new(home: PathBuf) -> Self {
        Self { home }
    }
    /// Resolve the current Windows user's known profile folder; no env override.
    pub fn current_user() -> Result<Self, ConfigError> {
        Ok(Self::new(current_home()?))
    }
}
impl CredentialReader for HomeFileReader {
    fn read(&self, key: PublicKey) -> Result<Option<String>, ConfigError> {
        let path = self.home.join(".env");
        super::files::plain(&path)
            .map_err(|_| ConfigError("configuration-home-file-unavailable"))?;
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(ConfigError("configuration-home-file-unavailable")),
        };
        if !file
            .metadata()
            .map_err(|_| ConfigError("configuration-home-file-unavailable"))?
            .is_file()
        {
            return Err(ConfigError("configuration-home-file-unavailable"));
        }
        let mut bytes = Vec::new();
        file.take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|_| ConfigError("configuration-home-file-unavailable"))?;
        super::files::plain(&path)
            .map_err(|_| ConfigError("configuration-home-file-unavailable"))?;
        parse(&bytes, key)
    }
}
#[cfg(not(windows))]
fn current_home() -> Result<PathBuf, ConfigError> {
    Err(ConfigError("configuration-home-unavailable"))
}
#[cfg(windows)]
fn current_home() -> Result<PathBuf, ConfigError> {
    use std::os::windows::ffi::OsStringExt;
    #[repr(C)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: *mut std::ffi::c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(pointer: *mut std::ffi::c_void);
        fn CoInitializeEx(reserved: *mut std::ffi::c_void, flags: u32) -> i32;
        fn CoUninitialize();
    }
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }
    struct Allocation(*mut u16);
    impl Drop for Allocation {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CoTaskMemFree(self.0.cast());
                }
            }
        }
    }
    // SAFETY: null reserved pointer and standard STA flag; balance only successful
    // initialization. An existing different apartment remains owned by its caller.
    let initialized = unsafe { CoInitializeEx(std::ptr::null_mut(), 2) };
    if initialized < 0 && initialized != 0x80010106u32 as i32 {
        return Err(ConfigError("configuration-home-unavailable"));
    }
    let _apartment = Apartment(initialized >= 0);
    let id = Guid {
        a: 0x5e6c858f,
        b: 0x0e22,
        c: 0x4760,
        d: [0x9a, 0xfe, 0xea, 0x33, 0x17, 0xb6, 0x71, 0x73],
    };
    let mut raw = std::ptr::null_mut();
    // SAFETY: fixed FOLDERID_Profile, current-caller null token, writable PWSTR.
    let status = unsafe { SHGetKnownFolderPath(&id, 0, std::ptr::null_mut(), &mut raw) };
    let allocation = Allocation(raw);
    if status < 0 || raw.is_null() {
        return Err(ConfigError("configuration-home-unavailable"));
    }
    let mut length = 0usize;
    // SAFETY: successful API returns NUL-terminated allocation, bounded to32768.
    while length < 32768 && unsafe { *raw.add(length) } != 0 {
        length += 1;
    }
    if length == 0 || length == 32768 {
        return Err(ConfigError("configuration-home-unavailable"));
    }
    // SAFETY: preceding bounded walk established live UTF16 units, copied before free.
    let home = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
        std::slice::from_raw_parts(allocation.0, length)
    }));
    if !home.is_absolute() {
        return Err(ConfigError("configuration-home-unavailable"));
    }
    Ok(home)
}
