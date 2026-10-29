// Temporary GitHub runner diagnostic. No app data, network, microphone or credentials.
#![cfg(windows)]
use std::{ffi::{c_void,OsStr},mem::{size_of,zeroed},os::windows::ffi::OsStrExt,ptr::{null, null_mut, read_unaligned}};
type Handle = *mut c_void;
#[repr(C)] struct SidAttr { sid:Handle, attributes:u32 }
#[repr(C)] struct StartupInfo { cb:u32, reserved:*mut u16, desktop:*mut u16, title:*mut u16, x:u32, y:u32, xsize:u32, ysize:u32, xchars:u32, ychars:u32, fill:u32, flags:u32, show:u16, cb_reserved:u16, reserved2:*mut u8, stdin:Handle, stdout:Handle, stderr:Handle }
#[repr(C)] struct ProcessInfo { process:Handle, thread:Handle, pid:u32, tid:u32 }
#[link(name="advapi32")] unsafe extern "system" {
 fn OpenProcessToken(p:Handle,access:u32,t:*mut Handle)->i32;
 fn CreateRestrictedToken(t:Handle,flags:u32,n:u32,s:Handle,np:u32,p:Handle,nr:u32,r:Handle,out:*mut Handle)->i32;
 fn GetTokenInformation(t:Handle,c:u32,b:Handle,len:u32,out:*mut u32)->i32;
 fn SetTokenInformation(t:Handle,c:u32,b:Handle,len:u32)->i32;
 fn GetSidSubAuthorityCount(s:Handle)->*const u8;
 fn GetSidSubAuthority(s:Handle,n:u32)->*const u32;
 fn GetLengthSid(s:Handle)->u32;
 fn ConvertStringSidToSidW(s:*const u16,out:*mut Handle)->i32;
 fn CreateProcessAsUserW(t:Handle,app:*const u16,cmd:*mut u16,pa:Handle,ta:Handle,inherit:i32,flags:u32,env:Handle,cwd:*const u16,si:*mut StartupInfo,pi:*mut ProcessInfo)->i32;
}
#[link(name="kernel32")] unsafe extern "system" {
 fn GetCurrentProcess()->Handle; fn GetLastError()->u32; fn CloseHandle(h:Handle)->i32;
 fn LocalFree(h:Handle)->Handle; fn WaitForSingleObject(h:Handle,ms:u32)->u32;
 fn GetExitCodeProcess(h:Handle,code:*mut u32)->i32; fn TerminateProcess(h:Handle,code:u32)->i32;
}
fn fail(stage:u32)->! { unsafe{eprintln!("error stage={} win32={}",stage,GetLastError())};std::process::exit(1) }
unsafe fn token_info(t:Handle)->(u32,u32,u32) {
 let mut flags=[0u32;2];let mut returned=0;
 for (i,c) in [20,26].into_iter().enumerate(){if unsafe{GetTokenInformation(t,c,(&mut flags[i] as *mut u32).cast(),4,&mut returned)}==0{fail(10+c)}}
 let mut buf=vec![0u8;4096];if unsafe{GetTokenInformation(t,25,buf.as_mut_ptr().cast(),4096,&mut returned)}==0{fail(35)}
 if returned<size_of::<SidAttr>() as u32{fail(36)}
 let a=unsafe{read_unaligned(buf.as_ptr().cast::<SidAttr>())};if a.sid.is_null(){fail(37)}
 let p=unsafe{GetSidSubAuthorityCount(a.sid)};if p.is_null() || unsafe{*p}==0{fail(38)}
 let q=unsafe{GetSidSubAuthority(a.sid,u32::from(*p-1))};if q.is_null(){fail(39)}
 (flags[0],flags[1],unsafe{read_unaligned(q)})
}
fn wide(s:&OsStr)->Vec<u16>{s.encode_wide().chain(Some(0)).collect()}
fn main(){unsafe{
 let mut original=null_mut();if OpenProcessToken(GetCurrentProcess(),0x8b,&mut original)==0{fail(1)}
 let current=token_info(original);
 if std::env::args().nth(1).as_deref()==Some("child") {
  println!("child elevation={} uiaccess={} rid={}",current.0,current.1,current.2);
  CloseHandle(original);if current!=(0,0,8192){std::process::exit(2)} return;
 }
 println!("parent elevation={} uiaccess={} rid={}",current.0,current.1,current.2);
 let mut restricted=null_mut();if CreateRestrictedToken(original,5,0,null_mut(),0,null_mut(),0,null_mut(),&mut restricted)==0{fail(2)}
 let before=token_info(restricted);println!("restricted_before elevation={} uiaccess={} rid={}",before.0,before.1,before.2);
 if before.2>8192 {
  let sidstr:Vec<u16>="S-1-16-8192".encode_utf16().chain(Some(0)).collect();let mut sid=null_mut();
  if ConvertStringSidToSidW(sidstr.as_ptr(),&mut sid)==0{fail(3)}
  let mut label=SidAttr{sid,attributes:0x20};let len=size_of::<SidAttr>() as u32+GetLengthSid(sid);
  if SetTokenInformation(restricted,25,(&mut label as *mut SidAttr).cast(),len)==0{fail(4)} LocalFree(sid);
 }
 let after=token_info(restricted);println!("restricted_after elevation={} uiaccess={} rid={}",after.0,after.1,after.2);
 if after!=(0,0,8192){eprintln!("error stage=5 win32=0");std::process::exit(1)}
 let exe=std::env::current_exe().unwrap_or_else(|_|fail(6));let app=wide(exe.as_os_str());
 let mut command:Vec<u16>=format!("\"{}\" child",exe.display()).encode_utf16().chain(Some(0)).collect();
 let mut env=[0u16,0u16];let mut startup:StartupInfo=zeroed();startup.cb=size_of::<StartupInfo>() as u32;let mut pi:ProcessInfo=zeroed();
 // Empty environment is deliberate. Only this program's fixed child is launched.
 if CreateProcessAsUserW(restricted,app.as_ptr(),command.as_mut_ptr(),null_mut(),null_mut(),0,0x400,env.as_mut_ptr().cast(),null(),&mut startup,&mut pi)==0{fail(7)}
 let wait=WaitForSingleObject(pi.process,10000);if wait!=0 {TerminateProcess(pi.process,1);WaitForSingleObject(pi.process,5000);eprintln!("error stage=8 win32={}",wait);std::process::exit(1)}
 let mut exit=99;if GetExitCodeProcess(pi.process,&mut exit)==0{fail(9)}
 println!("child_exit={}",exit);CloseHandle(pi.thread);CloseHandle(pi.process);CloseHandle(restricted);CloseHandle(original);
 if exit!=0{std::process::exit(1)}
}}
