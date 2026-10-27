//! Real Win32 paste and format preservation on a private, noninteractive window station.
//! The parent test never opens a clipboard. Its owned child changes station first,
//! and fails closed if isolation is unavailable. No interactive desktop is switched.
#[cfg(windows)]
#[path = "../../desktop/src-tauri/src/native_clipboard.rs"]
mod native_clipboard;

#[cfg(windows)]
mod windows {
    use super::native_clipboard;
    use notype_core::DeliveryOutcome;
    use std::{
        ffi::c_void,
        io::Read,
        process::{Command, Stdio},
        sync::{
            atomic::{AtomicBool, AtomicIsize, AtomicU8, Ordering},
            mpsc, Arc, Mutex, OnceLock,
        },
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn production_paste_preserves_formats_and_never_confirms_a_newer_clipboard() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "windows::isolated_clipboard_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("NOTYPE_CLIPBOARD_ISOLATED_CHILD", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("isolated clipboard child exceeded its deadline");
            }
            thread::sleep(Duration::from_millis(10));
        };
        let mut output = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut output)
            .unwrap();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut output)
            .unwrap();
        assert!(status.success(), "isolated child failed:\n{output}");
        assert!(
            output.contains("isolated clipboard: 4 cases passed"),
            "{output}"
        );
    }

    /// Entry point invoked explicitly by the parent, never against the interactive clipboard.
    #[test]
    #[ignore = "owned child entry point, executed explicitly by the parent test"]
    fn isolated_clipboard_child() {
        assert_eq!(
            std::env::var("NOTYPE_CLIPBOARD_ISOLATED_CHILD").as_deref(),
            Ok("1")
        );
        let isolated = Isolation::create();
        let state = Arc::new(FixtureState::default());
        assert!(STATE.set(state.clone()).is_ok());
        let desktop = isolated.desktop;
        let (tx, rx) = mpsc::sync_channel(1);
        let fixture = thread::spawn(move || unsafe {
            bind_desktop(desktop);
            let class = wide("STATIC");
            let window = CreateWindowExW(
                0,
                class.as_ptr(),
                std::ptr::null(),
                0,
                0,
                0,
                0,
                0,
                -3,
                0,
                0,
                std::ptr::null(),
            );
            assert_ne!(window, 0, "owned clipboard window creation failed");
            BASE_PROC.store(
                SetWindowLongPtrW(window, -4, fixture_proc as *const () as isize),
                Ordering::SeqCst,
            );
            assert_ne!(BASE_PROC.load(Ordering::SeqCst), 0);
            tx.send(window).unwrap();
            let mut message: Message = std::mem::zeroed();
            while GetMessageW(&mut message, 0, 0, 0) > 0 {
                DispatchMessageW(&message);
            }
        });
        let window = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let opaque_format =
            unsafe { RegisterClipboardFormatW(wide("notype-owned-opaque-fixture").as_ptr()) };
        assert_ne!(opaque_format, 0);
        let original = "original fixture 🧪";
        let opaque = [0, 255, 128, 42, 17];

        // Normal Unicode paste deliberately requests synthesized CF_TEXT. The
        // resulting clipboard sequence change must not invalidate ownership.
        set_clipboard(window, original, Some((opaque_format, &opaque)));
        let original_opaque = clipboard_bytes(window, opaque_format);
        assert!(original_opaque.starts_with(&opaque));
        state.mode.store(0, Ordering::SeqCst);
        let speech = "notype 中文 OpenAI 🧪\r\n第二行。";
        let observed = state.clone();
        assert_eq!(
            native_clipboard::paste(
                window,
                speech.into(),
                move || {
                    bind_desktop(desktop);
                    true
                },
                move || observed
                    .seen
                    .lock()
                    .unwrap()
                    .last()
                    .is_some_and(|text| text.len() == speech.len())
            ),
            DeliveryOutcome::Delivered
        );
        assert_eq!(state.seen.lock().unwrap().as_slice(), &[speech]);
        assert_eq!(clipboard_text(window), original);
        assert_eq!(clipboard_bytes(window, opaque_format), original_opaque);
        assert!(state.errors.lock().unwrap().is_empty());

        // The target copies NEW before reading the pending paste. OLD and NEW
        // have equal byte length, so metadata-only postconditions cannot tell.
        state.mode.store(1, Ordering::SeqCst);
        let observed = state.clone();
        assert_eq!(
            native_clipboard::paste(
                window,
                "OLD".into(),
                move || {
                    bind_desktop(desktop);
                    true
                },
                move || observed
                    .seen
                    .lock()
                    .unwrap()
                    .last()
                    .is_some_and(|text| text.len() == 3)
            ),
            DeliveryOutcome::Uncertain
        );
        assert_eq!(state.seen.lock().unwrap().last().unwrap(), "NEW");
        assert_eq!(
            clipboard_text(window),
            "NEW",
            "preserve newer owner, never restore stale backup"
        );

        let count = state.seen.lock().unwrap().len();
        assert_eq!(
            native_clipboard::paste(
                window,
                "never sent".into(),
                move || {
                    bind_desktop(desktop);
                    false
                },
                || true
            ),
            DeliveryOutcome::TargetChanged
        );
        assert_eq!(state.seen.lock().unwrap().len(), count);
        assert_eq!(clipboard_text(window), "NEW");

        // A committed slow handler cannot be replayed or have its source removed
        // at the caller deadline. Cleanup restores only after that handler ends.
        set_clipboard(window, original, Some((opaque_format, &opaque)));
        state.mode.store(2, Ordering::SeqCst);
        let observed = state.clone();
        assert_eq!(
            native_clipboard::paste(
                window,
                "SLOW".into(),
                move || {
                    bind_desktop(desktop);
                    true
                },
                move || observed
                    .seen
                    .lock()
                    .unwrap()
                    .last()
                    .is_some_and(|text| text == "SLOW")
            ),
            DeliveryOutcome::Uncertain
        );
        assert_eq!(state.seen.lock().unwrap().len(), count);
        state.slow_release.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if state.seen.lock().unwrap().len() == count + 1 && clipboard_text(window) == original {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "slow handler did not restore its original formats"
            );
            thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(clipboard_bytes(window, opaque_format), original_opaque);
        assert!(state.errors.lock().unwrap().is_empty());
        unsafe {
            assert_ne!(PostMessageW(window, 0x8001, 0, 0), 0);
        }
        fixture.join().unwrap();
        println!("isolated clipboard: 4 cases passed");
    }

    #[derive(Default)]
    struct FixtureState {
        mode: AtomicU8,
        slow_release: AtomicBool,
        seen: Mutex<Vec<String>>,
        errors: Mutex<Vec<String>>,
    }
    static STATE: OnceLock<Arc<FixtureState>> = OnceLock::new();
    static BASE_PROC: AtomicIsize = AtomicIsize::new(0);
    unsafe extern "system" fn fixture_proc(window: isize, id: u32, w: usize, l: isize) -> isize {
        if id == 0x0302 {
            let state = STATE.get().unwrap();
            let outcome = std::panic::catch_unwind(|| {
                match state.mode.load(Ordering::SeqCst) {
                    1 => set_clipboard(window, "NEW", None),
                    2 => {
                        while !state.slow_release.load(Ordering::Acquire) {
                            thread::sleep(Duration::from_millis(2));
                        }
                    }
                    _ => (),
                }
                let text = clipboard_text(window);
                // Trigger the Windows standard text-format synthesis path.
                with_clipboard(window, || unsafe {
                    assert_ne!(GetClipboardData(1), 0);
                });
                state.seen.lock().unwrap().push(text);
            });
            if outcome.is_err() {
                state
                    .errors
                    .lock()
                    .unwrap()
                    .push("owned paste handler failed".into());
            }
            return 0;
        }
        if id == 0x8001 {
            unsafe {
                DestroyWindow(window);
                PostQuitMessage(0);
            }
            return 0;
        }
        unsafe { CallWindowProcW(BASE_PROC.load(Ordering::SeqCst), window, id, w, l) }
    }

    struct Isolation {
        desktop: isize,
    }
    impl Isolation {
        fn create() -> Self {
            unsafe {
                // CREATE_ONLY forbids silently joining an existing station.
                let station_name =
                    wide(&format!("notype-clipboard-fixture-{}", std::process::id()));
                let station =
                    CreateWindowStationW(station_name.as_ptr(), 1, 0x000f037f, std::ptr::null());
                assert_ne!(
                    station,
                    0,
                    "private window station unavailable: {}",
                    GetLastError()
                );
                assert_ne!(
                    station,
                    GetProcessWindowStation(),
                    "isolation returned current station"
                );
                assert_ne!(
                    SetProcessWindowStation(station),
                    0,
                    "cannot bind private window station: {}",
                    GetLastError()
                );
                assert_eq!(GetProcessWindowStation(), station);
                let mut name = [0u16; 256];
                let mut needed = 0;
                assert_ne!(
                    GetUserObjectInformationW(
                        station,
                        2,
                        name.as_mut_ptr() as *mut _,
                        std::mem::size_of_val(&name) as u32,
                        &mut needed
                    ),
                    0
                );
                let name =
                    String::from_utf16_lossy(&name[..name.iter().position(|c| *c == 0).unwrap()]);
                assert!(
                    !name.eq_ignore_ascii_case("WinSta0"),
                    "interactive station forbidden"
                );
                let desktop = CreateDesktopW(
                    wide("Default").as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    0x000f01ff,
                    std::ptr::null(),
                );
                assert_ne!(
                    desktop,
                    0,
                    "private desktop unavailable: {}",
                    GetLastError()
                );
                bind_desktop(desktop);
                assert_eq!(GetThreadDesktop(GetCurrentThreadId()), desktop);
                // All subsequent clipboard access is guarded by this established
                // process station. Handles are destroyed when the child exits.
                Self { desktop }
            }
        }
    }
    fn bind_desktop(desktop: isize) {
        unsafe {
            if GetThreadDesktop(GetCurrentThreadId()) != desktop {
                assert_ne!(
                    SetThreadDesktop(desktop),
                    0,
                    "cannot bind isolated desktop: {}",
                    GetLastError()
                );
            }
        }
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn with_clipboard<T>(owner: isize, operation: impl FnOnce() -> T) -> T {
        let deadline = Instant::now() + Duration::from_secs(1);
        while unsafe { OpenClipboard(owner) } == 0 {
            assert!(Instant::now() < deadline, "owned clipboard lock timeout");
            thread::sleep(Duration::from_millis(2));
        }
        struct Close;
        impl Drop for Close {
            fn drop(&mut self) {
                unsafe {
                    CloseClipboard();
                }
            }
        }
        let _close = Close;
        operation()
    }
    fn publish_bytes(kind: u32, bytes: &[u8]) {
        unsafe {
            let memory = GlobalAlloc(0x42, bytes.len());
            assert_ne!(memory, 0);
            let pointer = GlobalLock(memory);
            assert!(!pointer.is_null());
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer as *mut u8, bytes.len());
            GlobalUnlock(memory);
            if SetClipboardData(kind, memory) == 0 {
                GlobalFree(memory);
                panic!("owned clipboard publish failed");
            }
        }
    }
    fn set_clipboard(owner: isize, text: &str, extra: Option<(u32, &[u8])>) {
        with_clipboard(owner, || unsafe {
            assert_ne!(EmptyClipboard(), 0);
            let bytes: Vec<u8> = wide(text).iter().flat_map(|c| c.to_le_bytes()).collect();
            publish_bytes(13, &bytes);
            if let Some((kind, bytes)) = extra {
                publish_bytes(kind, bytes);
            }
        });
    }
    fn clipboard_bytes(owner: isize, kind: u32) -> Vec<u8> {
        with_clipboard(owner, || unsafe {
            let memory = GetClipboardData(kind);
            assert_ne!(memory, 0);
            let size = GlobalSize(memory);
            assert!(size <= 1024 * 1024);
            let pointer = GlobalLock(memory);
            assert!(!pointer.is_null());
            let bytes = std::slice::from_raw_parts(pointer as *const u8, size).to_vec();
            GlobalUnlock(memory);
            bytes
        })
    }
    fn clipboard_text(owner: isize) -> String {
        let bytes = clipboard_bytes(owner, 13);
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .take_while(|c| *c != 0)
            .collect();
        String::from_utf16(&units).unwrap()
    }
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct Message {
        window: isize,
        id: u32,
        w: usize,
        l: isize,
        time: u32,
        point: Point,
        private: u32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn CreateWindowStationW(
            name: *const u16,
            flags: u32,
            access: u32,
            security: *const c_void,
        ) -> isize;
        fn GetProcessWindowStation() -> isize;
        fn SetProcessWindowStation(station: isize) -> i32;
        fn GetUserObjectInformationW(
            handle: isize,
            index: i32,
            data: *mut c_void,
            length: u32,
            needed: *mut u32,
        ) -> i32;
        fn CreateDesktopW(
            name: *const u16,
            device: *const u16,
            mode: *const c_void,
            flags: u32,
            access: u32,
            security: *const c_void,
        ) -> isize;
        fn GetThreadDesktop(thread: u32) -> isize;
        fn SetThreadDesktop(desktop: isize) -> i32;
        fn CreateWindowExW(
            ex: u32,
            class: *const u16,
            name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: isize,
            menu: isize,
            instance: isize,
            param: *const c_void,
        ) -> isize;
        fn SetWindowLongPtrW(window: isize, index: i32, value: isize) -> isize;
        fn CallWindowProcW(proc: isize, window: isize, id: u32, w: usize, l: isize) -> isize;
        fn GetMessageW(message: *mut Message, window: isize, min: u32, max: u32) -> i32;
        fn DispatchMessageW(message: *const Message) -> isize;
        fn PostMessageW(window: isize, id: u32, w: usize, l: isize) -> i32;
        fn PostQuitMessage(exit: i32);
        fn DestroyWindow(window: isize) -> i32;
        fn RegisterClipboardFormatW(name: *const u16) -> u32;
        fn OpenClipboard(owner: isize) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn GetClipboardData(kind: u32) -> isize;
        fn SetClipboardData(kind: u32, data: isize) -> isize;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThreadId() -> u32;
        fn GetLastError() -> u32;
        fn GlobalAlloc(flags: u32, bytes: usize) -> isize;
        fn GlobalLock(memory: isize) -> *mut c_void;
        fn GlobalUnlock(memory: isize) -> i32;
        fn GlobalSize(memory: isize) -> usize;
        fn GlobalFree(memory: isize) -> isize;
    }
}
