use std::{ffi::{c_void, OsString}, os::windows::ffi::OsStringExt, path::PathBuf};
#[repr(C)] struct Guid { d1:u32, d2:u16, d3:u16, d4:[u8;8] }
#[link(name="shell32")] unsafe extern "system" { fn SHGetKnownFolderPath(id:*const Guid, flags:u32, token:*mut c_void, path:*mut *mut u16)->i32; }
#[link(name="ole32")] unsafe extern "system" { fn CoTaskMemFree(p:*mut c_void); }
fn main() {
    let id=Guid{d1:0x3eb685db,d2:0x65f9,d3:0x4cf6,d4:[0xa0,0x3a,0xe3,0xef,0x65,0x72,0x9f,0x3d]};
    let mut ptr=std::ptr::null_mut();
    let rc=unsafe { SHGetKnownFolderPath(&id,0,std::ptr::null_mut(),&mut ptr) };
    if std::env::args().nth(1).as_deref()==Some("missing") {
        if !ptr.is_null() { unsafe { CoTaskMemFree(ptr.cast()) } }
        std::process::exit(if rc == 0x80070003u32 as i32 {0} else {10});
    }
    if rc!=0 || ptr.is_null() { std::process::exit(11); }
    let mut n=0; unsafe { while *ptr.add(n)!=0 { n+=1; } }
    let actual=PathBuf::from(OsString::from_wide(unsafe { std::slice::from_raw_parts(ptr,n) }));
    unsafe { CoTaskMemFree(ptr.cast()) };
    let expected=PathBuf::from(std::env::var_os("USERPROFILE").unwrap()).join("AppData").join("Roaming");
    let same=std::fs::canonicalize(actual).ok().zip(std::fs::canonicalize(expected).ok()).is_some_and(|(a,b)| a==b);
    std::process::exit(if same {0} else {12});
}
