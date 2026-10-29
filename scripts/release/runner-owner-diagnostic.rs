#![cfg(windows)]
// Temporary, read-only runner owner diagnostic. No SID, path or account is printed.
use std::{ffi::{c_void,OsStr},mem::size_of,os::windows::{ffi::OsStrExt,fs::MetadataExt},path::{Path,PathBuf},ptr::{null_mut,read_unaligned}};
type H=*mut c_void;
#[repr(C)] struct SidAttr{sid:H,attributes:u32}
#[link(name="advapi32")] unsafe extern "system" {
 fn OpenProcessToken(p:H,access:u32,t:*mut H)->i32;fn GetTokenInformation(t:H,class:u32,buffer:H,len:u32,ret:*mut u32)->i32;
 fn GetNamedSecurityInfoW(name:*const u16,kind:u32,info:u32,owner:*mut H,group:*mut H,dacl:*mut H,sacl:*mut H,descriptor:*mut H)->u32;
 fn ConvertStringSidToSidW(s:*const u16,out:*mut H)->i32;fn EqualSid(a:H,b:H)->i32;
}
#[link(name="kernel32")] unsafe extern "system" {fn GetCurrentProcess()->H;fn GetLastError()->u32;fn CloseHandle(h:H)->i32;fn LocalFree(h:H)->H;}
fn fail(stage:u32,code:u32)->!{eprintln!("error stage={stage} code={code}");std::process::exit(1)}
fn wide(s:&OsStr)->Vec<u16>{s.encode_wide().chain(Some(0)).collect()}
fn check_path(base:&Path,p:&Path){let mut current=base.to_path_buf();let meta=std::fs::symlink_metadata(&current).unwrap_or_else(|_|fail(1,0));if meta.file_attributes()&0x400!=0{fail(2,0)}for c in p.strip_prefix(base).unwrap_or_else(|_|fail(3,0)).components(){current.push(c);let m=std::fs::symlink_metadata(&current).unwrap_or_else(|_|fail(4,0));if m.file_attributes()&0x400!=0{fail(5,0)}}}
fn main(){unsafe{
 if std::env::args_os().len()!=1{fail(6,0)}
 let base=PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap_or_else(||fail(7,0)));if !base.is_absolute(){fail(8,0)}
 let paths=[("data",base.join("notype-native-smoke")),("extracted",base.join("notype-assembled/extracted"))];
 for (_,p) in &paths{check_path(&base,p);if !std::fs::metadata(p).map(|m|m.is_dir()).unwrap_or(false){fail(16,0)}}
 let mut token=null_mut();if OpenProcessToken(GetCurrentProcess(),8,&mut token)==0{fail(9,GetLastError())}
 let mut user=vec![0u8;4096];let mut owner=vec![0u8;4096];let mut returned=0;
 if GetTokenInformation(token,1,user.as_mut_ptr().cast(),4096,&mut returned)==0||returned<size_of::<SidAttr>() as u32{fail(10,GetLastError())}
 if GetTokenInformation(token,4,owner.as_mut_ptr().cast(),4096,&mut returned)==0||returned<size_of::<H>() as u32{fail(11,GetLastError())}
 let user_sid=read_unaligned(user.as_ptr().cast::<SidAttr>()).sid;let owner_sid=read_unaligned(owner.as_ptr().cast::<H>());
 if user_sid.is_null()||owner_sid.is_null(){fail(12,0)}
 let admins:Vec<u16>="S-1-5-32-544".encode_utf16().chain(Some(0)).collect();let mut admin_sid=null_mut();if ConvertStringSidToSidW(admins.as_ptr(),&mut admin_sid)==0{fail(13,GetLastError())}
 for (label,p) in paths {let name=wide(p.as_os_str());let mut file_owner=null_mut();let mut descriptor=null_mut();
  let rc=GetNamedSecurityInfoW(name.as_ptr(),1,1,&mut file_owner,null_mut(),null_mut(),null_mut(),&mut descriptor);
  if rc!=0{fail(14,rc)}if file_owner.is_null()||descriptor.is_null(){fail(15,0)}
  println!("owner {} user={} token_owner={} administrators={}",label,u8::from(EqualSid(file_owner,user_sid)!=0),u8::from(EqualSid(file_owner,owner_sid)!=0),u8::from(EqualSid(file_owner,admin_sid)!=0));LocalFree(descriptor);
 }
 LocalFree(admin_sid);CloseHandle(token);
}}
