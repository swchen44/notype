use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct CpuCapabilities {
    pub avx2: bool,
    pub bmi2: bool,
}
#[derive(Serialize)]
pub struct HostReport {
    pub schema: u32,
    pub platform: &'static str,
    pub arch: &'static str,
    pub integrity: &'static str,
    pub cpu: CpuCapabilities,
    pub roaming_app_data: Option<PathBuf>,
}

/// Uses only baseline Rust/std and Windows system APIs. Never links ORT, ggml,
/// a WebView runtime or application initialization into the pre-spawn probe.
pub fn host_report() -> HostReport {
    #[cfg(target_arch = "x86_64")]
    let cpu = CpuCapabilities {
        avx2: std::is_x86_feature_detected!("avx2"),
        bmi2: std::is_x86_feature_detected!("bmi2"),
    };
    #[cfg(not(target_arch = "x86_64"))]
    let cpu = CpuCapabilities {
        avx2: false,
        bmi2: false,
    };
    HostReport {
        schema: 1,
        platform: if cfg!(windows) {
            "win32"
        } else {
            std::env::consts::OS
        },
        arch: if cfg!(target_arch = "x86_64") {
            "x64"
        } else {
            std::env::consts::ARCH
        },
        integrity: if normal_user() {
            "medium"
        } else {
            "unsupported"
        },
        cpu,
        roaming_app_data: roaming_app_data(),
    }
}
#[cfg(not(windows))]
fn normal_user() -> bool {
    false
}
#[cfg(not(windows))]
fn roaming_app_data() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    #[link(name = "advapi32")]
    extern "system" {
        pub fn OpenProcessToken(process: *mut c_void, access: u32, token: *mut *mut c_void) -> i32;
        pub fn GetTokenInformation(
            token: *mut c_void,
            class: i32,
            information: *mut c_void,
            length: u32,
            returned: *mut u32,
        ) -> i32;
        pub fn GetSidSubAuthorityCount(sid: *const c_void) -> *const u8;
        pub fn GetSidSubAuthority(sid: *const c_void, index: u32) -> *const u32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetCurrentProcess() -> *mut c_void;
    }
    #[repr(C)]
    pub struct SidAndAttributes {
        pub sid: *const c_void,
        pub attributes: u32,
    }
    #[repr(C)]
    pub struct Guid {
        pub a: u32,
        pub b: u16,
        pub c: u16,
        pub d: [u8; 8],
    }
    #[link(name = "shell32")]
    extern "system" {
        pub fn SHGetKnownFolderPath(
            folder: *const Guid,
            flags: u32,
            token: *mut c_void,
            result: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        pub fn CoTaskMemFree(pointer: *mut c_void);
    }
}
#[cfg(windows)]
fn normal_user() -> bool {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::ptr::{null_mut, read_unaligned};
    use windows::*;
    unsafe {
        let mut raw = null_mut();
        if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut raw) == 0 {
            return false;
        }
        let token = OwnedHandle::from_raw_handle(raw);
        let mut returned = 0;
        for class in [20, 26] {
            // TokenElevation and TokenUIAccess
            let mut flag = 1u32;
            if GetTokenInformation(
                token.as_raw_handle(),
                class,
                (&mut flag as *mut u32).cast(),
                4,
                &mut returned,
            ) == 0
                || flag != 0
            {
                return false;
            }
        }
        GetTokenInformation(token.as_raw_handle(), 25, null_mut(), 0, &mut returned);
        if returned < std::mem::size_of::<SidAndAttributes>() as u32 || returned > 4096 {
            return false;
        }
        let mut buffer = vec![0u8; returned as usize];
        if GetTokenInformation(
            token.as_raw_handle(),
            25,
            buffer.as_mut_ptr().cast(),
            returned,
            &mut returned,
        ) == 0
        {
            return false;
        }
        let label = read_unaligned(buffer.as_ptr().cast::<SidAndAttributes>());
        if label.sid.is_null() {
            return false;
        }
        let count_pointer = GetSidSubAuthorityCount(label.sid);
        if count_pointer.is_null() || *count_pointer == 0 {
            return false;
        }
        let rid_pointer = GetSidSubAuthority(label.sid, u32::from(*count_pointer - 1));
        !rid_pointer.is_null() && read_unaligned(rid_pointer) == 8192
    }
}
#[cfg(windows)]
fn roaming_app_data() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::null_mut;
    use windows::*;
    const ROAMING: Guid = Guid {
        a: 0x3eb685db,
        b: 0x65f9,
        c: 0x4cf6,
        d: [0xa0, 0x3a, 0xe3, 0xef, 0x65, 0x72, 0x9f, 0x3d],
    };
    unsafe {
        let mut result = null_mut();
        if SHGetKnownFolderPath(&ROAMING, 0, null_mut(), &mut result) < 0 || result.is_null() {
            return None;
        }
        let mut length = 0;
        while length < 32768 && *result.add(length) != 0 {
            length += 1;
        }
        let value = if length < 32768 {
            Some(PathBuf::from(std::ffi::OsString::from_wide(
                std::slice::from_raw_parts(result, length),
            )))
        } else {
            None
        };
        CoTaskMemFree(result.cast());
        value
    }
}
