//! Captures only application metadata and opaque original-control identities.
//! Native handles stay in memory; no title, field value, or Enter API.
use notype_core::{DeliveryAdapter, DeliveryFuture, DeliveryOutcome, DeliveryRequest};
#[cfg(windows)]
#[path = "native_clipboard.rs"]
mod clipboard;
#[path = "native_target_policy.rs"]
mod target_policy;
#[cfg(windows)]
use std::sync::Arc;

#[derive(Default)]
pub struct InputCapture {
    pub target: Option<String>,
    pub application: Option<String>,
}

/// Debug QA may capture only the exact process launched by its owned fixture harness.
#[cfg(debug_assertions)]
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebugFixtureOwner {
    pub pid: u32,
    pub image: String,
}

#[derive(Clone)]
pub struct NativeInput {
    #[cfg(windows)]
    inner: Arc<platform::Registry>,
}
impl Default for NativeInput {
    fn default() -> Self {
        Self {
            #[cfg(windows)]
            inner: Arc::new(platform::Registry::new()),
        }
    }
}
impl NativeInput {
    #[cfg(debug_assertions)]
    pub fn capture_owned_fixture(
        &self,
        owner: DebugFixtureOwner,
        require_target: bool,
    ) -> Result<InputCapture, String> {
        #[cfg(windows)]
        {
            self.inner.capture_fixture(owner, require_target)
        }
        #[cfg(not(windows))]
        {
            let _ = (owner, require_target);
            Err("owned-fixture-platform-unsupported".into())
        }
    }
    pub fn capture(&self) -> InputCapture {
        #[cfg(windows)]
        {
            self.inner.capture()
        }
        #[cfg(not(windows))]
        {
            InputCapture::default()
        }
    }
    pub fn available(&self) -> bool {
        #[cfg(windows)]
        {
            platform::watch_ready()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}
impl DeliveryAdapter for NativeInput {
    fn deliver(&self, request: DeliveryRequest) -> DeliveryFuture {
        let input = self.clone();
        Box::pin(async move {
            #[cfg(windows)]
            {
                input.inner.deliver(request)
            }
            #[cfg(not(windows))]
            {
                let _ = (input, request);
                DeliveryOutcome::Unavailable
            }
        })
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicBool, AtomicU64, Ordering},
            Mutex, OnceLock,
        },
        time::{SystemTime, UNIX_EPOCH},
    };
    use win::*;
    static FOCUS_EPOCH: AtomicU64 = AtomicU64::new(0);
    static READY: AtomicBool = AtomicBool::new(false);
    static WATCH: OnceLock<()> = OnceLock::new();

    unsafe extern "system" fn focus_event(
        _: isize,
        _: u32,
        _: isize,
        _: i32,
        _: i32,
        _: u32,
        _: u32,
    ) {
        FOCUS_EPOCH.fetch_add(1, Ordering::SeqCst);
    }
    pub fn watch_ready() -> bool {
        READY.load(Ordering::SeqCst)
    }
    fn start_watch() {
        WATCH.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || unsafe {
                let foreground = SetWinEventHook(3, 3, 0, Some(focus_event), 0, 0, 0);
                let focus = SetWinEventHook(0x8005, 0x8005, 0, Some(focus_event), 0, 0, 0);
                let ready = foreground != 0 && focus != 0;
                READY.store(ready, Ordering::SeqCst);
                let _ = tx.send(());
                if ready {
                    let mut message: Msg = std::mem::zeroed();
                    while GetMessageW(&mut message, 0, 0, 0) > 0 {
                        DispatchMessageW(&message);
                    }
                }
                READY.store(false, Ordering::SeqCst);
                if foreground != 0 {
                    UnhookWinEvent(foreground);
                }
                if focus != 0 {
                    UnhookWinEvent(focus);
                }
            });
            let _ = rx.recv_timeout(std::time::Duration::from_secs(2));
        });
    }
    #[derive(Clone, PartialEq, Eq)]
    enum TargetScope {
        Production,
        #[cfg(debug_assertions)]
        Fixture {
            pid: u32,
            created: u64,
            image: String,
        },
    }
    impl TargetScope {
        fn matches(&self, _pid: u32) -> bool {
            match self {
                Self::Production => true,
                #[cfg(debug_assertions)]
                Self::Fixture {
                    pid: expected,
                    created,
                    image,
                } => {
                    _pid == *expected
                        && process_details(_pid).is_some_and(|(actual_created, actual_image)| {
                            actual_created == *created && actual_image.eq_ignore_ascii_case(image)
                        })
                }
            }
        }
    }
    #[derive(Clone, PartialEq, Eq)]
    struct Target {
        foreground: isize,
        control: isize,
        pid: u32,
        created: u64,
        selection: (u32, u32),
        length: usize,
        style: isize,
        epoch: u64,
        scope: TargetScope,
        kind: target_policy::ControlKind,
        document: usize,
        code_page: usize,
        eol: usize,
        convert_endings: bool,
    }
    pub struct Registry {
        targets: Mutex<HashMap<String, Target>>,
        nonce: u128,
        next: AtomicU64,
    }
    impl Registry {
        pub fn new() -> Self {
            start_watch();
            Self {
                targets: Mutex::new(HashMap::new()),
                nonce: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
                next: AtomicU64::new(0),
            }
        }
        pub fn capture(&self) -> InputCapture {
            self.capture_scoped(TargetScope::Production)
        }
        #[cfg(debug_assertions)]
        pub fn capture_fixture(
            &self,
            owner: DebugFixtureOwner,
            require_target: bool,
        ) -> Result<InputCapture, String> {
            if owner.pid == 0
                || !std::path::Path::new(&owner.image).is_absolute()
                || !std::path::Path::new(&owner.image)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("NativeInputFixture.exe"))
            {
                return Err("owned-fixture-identity-required".into());
            }
            let (created, image) = process_details(owner.pid)
                .filter(|(_, image)| image.eq_ignore_ascii_case(&owner.image))
                .ok_or("owned-fixture-identity-mismatch")?;
            let scope = TargetScope::Fixture {
                pid: owner.pid,
                created,
                image,
            };
            let foreground = unsafe { GetForegroundWindow() };
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(foreground, &mut pid) };
            if !scope.matches(pid) {
                return Err("owned-fixture-not-foreground".into());
            }
            let capture = self.capture_scoped(scope);
            if require_target && capture.target.is_none() {
                return Err("owned-fixture-target-unavailable".into());
            }
            Ok(capture)
        }
        fn capture_scoped(&self, scope: TargetScope) -> InputCapture {
            let foreground = unsafe { GetForegroundWindow() };
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(foreground, &mut pid) };
            if !scope.matches(pid) {
                return InputCapture::default();
            }
            let application = application_name(foreground);
            let Some(target) = snapshot(&scope) else {
                return InputCapture {
                    target: None,
                    application,
                };
            };
            if target.foreground != foreground {
                return InputCapture::default();
            }
            let token = format!(
                "input-{:x}-{}",
                self.nonce,
                self.next.fetch_add(1, Ordering::Relaxed)
            );
            let Ok(mut targets) = self.targets.lock() else {
                return InputCapture {
                    target: None,
                    application,
                };
            };
            // A bounded process-local registry never revives persisted HWNDs after restart.
            if targets.len() >= 256 {
                targets.clear();
            }
            targets.insert(token.clone(), target);
            InputCapture {
                target: Some(token),
                application,
            }
        }
        pub fn deliver(&self, request: DeliveryRequest) -> DeliveryOutcome {
            let target = self
                .targets
                .lock()
                .ok()
                .and_then(|mut targets| targets.remove(&request.original_target));
            let Some(target) = target else {
                return DeliveryOutcome::Unavailable;
            };
            if request.text.is_empty()
                || request.text.contains('\0')
                || request.text.encode_utf16().count() > 1_000_000
            {
                return DeliveryOutcome::Unavailable;
            }
            let Some(current) = snapshot(&target.scope) else {
                return DeliveryOutcome::TargetChanged;
            };
            if current != target {
                return DeliveryOutcome::TargetChanged;
            }
            if target.kind == target_policy::ControlKind::Scintilla {
                let Some(text) = target_policy::scintilla_text(
                    &request.text,
                    target.code_page,
                    target.eol,
                    target.convert_endings,
                ) else {
                    return DeliveryOutcome::Unavailable;
                };
                let Some(expected_length) = target
                    .length
                    .checked_sub((target.selection.1 - target.selection.0) as usize)
                    .and_then(|n| n.checked_add(text.len()))
                else {
                    return DeliveryOutcome::Unavailable;
                };
                let expected_caret = target.selection.0 as usize + text.len();
                if expected_caret > u32::MAX as usize || expected_length > u32::MAX as usize {
                    return DeliveryOutcome::Unavailable;
                }
                let before = target.clone();
                let after = target.clone();
                return clipboard::paste(
                    target.control,
                    text,
                    move || snapshot(&before.scope).as_ref() == Some(&before),
                    move || {
                        snapshot(&after.scope).is_some_and(|actual| {
                            actual.foreground == after.foreground
                                && actual.control == after.control
                                && actual.pid == after.pid
                                && actual.created == after.created
                                && actual.epoch == after.epoch
                                && actual.document == after.document
                                && actual.code_page == after.code_page
                                && actual.eol == after.eol
                                && actual.convert_endings == after.convert_endings
                                && actual.length == expected_length
                                && actual.selection
                                    == (expected_caret as u32, expected_caret as u32)
                        })
                    },
                );
            }
            if target.style & 0x4 == 0 && request.text.contains(['\r', '\n']) {
                return DeliveryOutcome::Unavailable;
            }
            let Some(limit) = message(target.control, 0xd5, 0, 0) else {
                return DeliveryOutcome::Unavailable;
            };
            let insertion = request.text.encode_utf16().count();
            let expected_length = target
                .length
                .saturating_sub((target.selection.1 - target.selection.0) as usize)
                .saturating_add(insertion);
            if expected_length > limit || expected_length > i32::MAX as usize {
                return DeliveryOutcome::Unavailable;
            }
            let mut text: Vec<u16> = request.text.encode_utf16().collect();
            text.push(0);
            if snapshot(&target.scope).as_ref() != Some(&target) {
                return DeliveryOutcome::TargetChanged;
            }
            // Directed Unicode insertion targets the captured control, never current focus.
            if message(target.control, 0xc2, 1, text.as_ptr() as isize).is_none() {
                return DeliveryOutcome::Uncertain;
            }
            let Some(after) = snapshot(&target.scope) else {
                return DeliveryOutcome::Uncertain;
            };
            let expected_caret = target.selection.0 as usize + insertion;
            if after.foreground == target.foreground
                && after.control == target.control
                && after.pid == target.pid
                && after.created == target.created
                && after.epoch == target.epoch
                && after.length == expected_length
                && after.selection == (expected_caret as u32, expected_caret as u32)
            {
                DeliveryOutcome::Delivered
            } else {
                DeliveryOutcome::Uncertain
            }
        }
    }
    fn message(window: isize, id: u32, w: usize, l: isize) -> Option<usize> {
        let mut result = 0;
        let success =
            unsafe { SendMessageTimeoutW(window, id, w, l, 0x1 | 0x2 | 0x20, 100, &mut result) };
        (success != 0).then_some(result)
    }
    fn snapshot(scope: &TargetScope) -> Option<Target> {
        unsafe {
            if !watch_ready() {
                return None;
            }
            let epoch = FOCUS_EPOCH.load(Ordering::SeqCst);
            let foreground = GetForegroundWindow();
            if foreground == 0 {
                return None;
            }
            let mut root_pid = 0;
            let thread = GetWindowThreadProcessId(foreground, &mut root_pid);
            if !scope.matches(root_pid) {
                return None;
            }
            let mut gui: GuiThreadInfo = std::mem::zeroed();
            gui.size = std::mem::size_of::<GuiThreadInfo>() as u32;
            if GetGUIThreadInfo(thread, &mut gui) == 0 || gui.focus == 0 {
                return None;
            }
            let control = gui.focus;
            if control != foreground && IsChild(foreground, control) == 0 {
                return None;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(control, &mut pid);
            if !scope.matches(pid) {
                return None;
            }
            if IsWindowEnabled(control) == 0 || IsWindowVisible(control) == 0 {
                return None;
            }
            let mut class = [0u16; 64];
            let size = GetClassNameW(control, class.as_mut_ptr(), 64);
            let class = String::from_utf16(class.get(..size.max(0) as usize)?).ok()?;
            let (created, image) = process_identity(pid)?;
            let kind = super::target_policy::control_kind(&class, &image)?;
            let style = GetWindowLongPtrW(control, -16);
            // Read-only EM_REPLACESEL can still write; password fields are never supported.
            if kind == target_policy::ControlKind::Edit
                && (IsWindowUnicode(control) == 0 || style & (0x800 | 0x20) != 0)
            {
                return None;
            }
            let mut start = 0u32;
            let mut end = 0u32;
            let (length, document, code_page, eol, convert_endings) =
                if kind == target_policy::ControlKind::Scintilla {
                    if message(control, 2140, 0, 0)? != 0
                        || message(control, 2570, 0, 0)? != 1
                        || message(control, 2423, 0, 0)? != 0
                        || message(control, 2581, 0, 0)? != 0
                        || message(control, 2583, 0, 0)? != 0
                    {
                        return None;
                    }
                    start = u32::try_from(message(control, 2143, 0, 0)?).ok()?;
                    end = u32::try_from(message(control, 2145, 0, 0)?).ok()?;
                    let document = message(control, 2357, 0, 0)?;
                    if document == 0 {
                        return None;
                    }
                    (
                        message(control, 2006, 0, 0)?,
                        document,
                        message(control, 2137, 0, 0)?,
                        message(control, 2030, 0, 0)?,
                        message(control, 2468, 0, 0)? != 0,
                    )
                } else {
                    message(
                        control,
                        0xb0,
                        &mut start as *mut u32 as usize,
                        &mut end as *mut u32 as isize,
                    )?;
                    (message(control, 0xe, 0, 0)?, 0, 0, 0, false)
                };
            if start > end || end as usize > length {
                return None;
            }
            if epoch != FOCUS_EPOCH.load(Ordering::SeqCst) || foreground != GetForegroundWindow() {
                return None;
            }
            Some(Target {
                foreground,
                control,
                pid,
                created,
                selection: (start, end),
                length,
                style,
                epoch,
                scope: scope.clone(),
                kind,
                document,
                code_page,
                eol,
                convert_endings,
            })
        }
    }
    fn process_identity(pid: u32) -> Option<(u64, String)> {
        let (created, image) = process_details(pid)?;
        let basename = image
            .rsplit(['\\', '/'])
            .next()?
            .strip_suffix(".exe")
            .unwrap_or_else(|| image.rsplit(['\\', '/']).next().unwrap_or(""));
        if basename.is_empty() || basename.len() > 100 || basename.chars().any(char::is_control) {
            return None;
        }
        Some((created, basename.to_owned()))
    }
    fn process_details(pid: u32) -> Option<(u64, String)> {
        unsafe {
            let process = OpenProcess(0x1000, 0, pid);
            if process == 0 {
                return None;
            }
            let mut times = [0u64; 4];
            let timed = GetProcessTimes(
                process,
                times.as_mut_ptr(),
                times.as_mut_ptr().add(1),
                times.as_mut_ptr().add(2),
                times.as_mut_ptr().add(3),
            );
            let mut image = vec![0u16; 32768];
            let mut size = image.len() as u32;
            let named = QueryFullProcessImageNameW(process, 0, image.as_mut_ptr(), &mut size);
            CloseHandle(process);
            if timed == 0 || named == 0 {
                return None;
            }
            let image = String::from_utf16(&image[..size as usize]).ok()?;
            Some((times[0], image))
        }
    }
    fn application_name(window: isize) -> Option<String> {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut pid);
        }
        let (_, name) = process_identity(pid)?;
        if [
            "applicationframehost",
            "runtimebroker",
            "wwahost",
            "dllhost",
            "notype",
        ]
        .iter()
        .any(|host| name.eq_ignore_ascii_case(host))
        {
            return None;
        }
        Some(name)
    }
}

#[cfg(windows)]
pub(crate) mod win {
    #[repr(C)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }
    #[repr(C)]
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }
    #[repr(C)]
    pub struct Msg {
        pub window: isize,
        pub message: u32,
        pub w: usize,
        pub l: isize,
        pub time: u32,
        pub point: Point,
        pub private: u32,
    }
    #[repr(C)]
    pub struct GuiThreadInfo {
        pub size: u32,
        pub flags: u32,
        pub active: isize,
        pub focus: isize,
        pub capture: isize,
        pub menu: isize,
        pub move_size: isize,
        pub caret: isize,
        pub caret_rect: Rect,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn GetForegroundWindow() -> isize;
        pub fn GetWindowThreadProcessId(window: isize, pid: *mut u32) -> u32;
        pub fn GetGUIThreadInfo(thread: u32, info: *mut GuiThreadInfo) -> i32;
        pub fn GetClassNameW(window: isize, class: *mut u16, max: i32) -> i32;
        pub fn GetWindowLongPtrW(window: isize, index: i32) -> isize;
        pub fn IsChild(parent: isize, child: isize) -> i32;
        pub fn IsWindowUnicode(window: isize) -> i32;
        pub fn IsWindowEnabled(window: isize) -> i32;
        pub fn IsWindowVisible(window: isize) -> i32;
        pub fn SendMessageTimeoutW(
            window: isize,
            message: u32,
            w: usize,
            l: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
        pub fn SetWinEventHook(
            min: u32,
            max: u32,
            module: isize,
            callback: Option<unsafe extern "system" fn(isize, u32, isize, i32, i32, u32, u32)>,
            process: u32,
            thread: u32,
            flags: u32,
        ) -> isize;
        pub fn UnhookWinEvent(hook: isize) -> i32;
        pub fn GetMessageW(message: *mut Msg, window: isize, min: u32, max: u32) -> i32;
        pub fn DispatchMessageW(message: *const Msg) -> isize;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
        pub fn CloseHandle(handle: isize) -> i32;
        pub fn GetProcessTimes(
            process: isize,
            created: *mut u64,
            exit: *mut u64,
            kernel: *mut u64,
            user: *mut u64,
        ) -> i32;
        pub fn QueryFullProcessImageNameW(
            process: isize,
            flags: u32,
            name: *mut u16,
            size: *mut u32,
        ) -> i32;
    }
}
