// Temporary GitHub runner diagnostic. No app data, network, microphone or credentials.
#![cfg(windows)]
use std::{ffi::{c_void,OsStr},mem::{size_of,zeroed},os::windows::{ffi::OsStrExt,fs::MetadataExt},ptr::{null, null_mut, read_unaligned}};
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

#[repr(C)] struct BasicLimits { per_process:i64, per_job:i64, flags:u32, min_ws:usize, max_ws:usize, active:u32, affinity:usize, priority:u32, scheduling:u32 }
#[repr(C)] struct IoCounters { read_ops:u64, write_ops:u64, other_ops:u64, read_bytes:u64, write_bytes:u64, other_bytes:u64 }
#[repr(C)] struct ExtendedLimits { basic:BasicLimits, io:IoCounters, process_mem:usize, job_mem:usize, peak_process:usize, peak_job:usize }
#[link(name="kernel32")] unsafe extern "system" {
 fn CreateJobObjectW(attr:Handle,name:*const u16)->Handle;
 fn SetInformationJobObject(job:Handle,class:u32,info:Handle,len:u32)->i32;
 fn AssignProcessToJobObject(job:Handle,process:Handle)->i32;
 fn TerminateJobObject(job:Handle,code:u32)->i32;
 fn ResumeThread(thread:Handle)->u32;
}
#[link(name="advapi32")] unsafe extern "system" {
 fn EqualSid(a:Handle,b:Handle)->i32;
 fn GetNamedSecurityInfoW(name:*const u16,object_type:u32,info:u32,owner:*mut Handle,group:*mut Handle,dacl:*mut Handle,sacl:*mut Handle,descriptor:*mut Handle)->u32;
 fn LookupPrivilegeValueW(system:*const u16,name:*const u16,luid:*mut Luid)->i32;
}
#[repr(C)] #[derive(Copy,Clone,PartialEq)] struct Luid { low:u32, high:i32 }
#[repr(C)] #[derive(Copy,Clone)] struct LuidAttributes { luid:Luid, attributes:u32 }
unsafe fn safe_token(t:Handle)->bool {
 let adminstr:Vec<u16>="S-1-5-32-544".encode_utf16().chain(Some(0)).collect();let mut admin=null_mut();
 if unsafe{ConvertStringSidToSidW(adminstr.as_ptr(),&mut admin)}==0{return false}
 let mut returned=0;let mut groups=vec![0u8;65536];
 if unsafe{GetTokenInformation(t,2,groups.as_mut_ptr().cast(),groups.len() as u32,&mut returned)}==0 {unsafe{LocalFree(admin)};return false}
 let count=unsafe{read_unaligned(groups.as_ptr().cast::<u32>())} as usize;
 let offset=std::mem::align_of::<SidAttr>();if count>(returned as usize).saturating_sub(offset)/size_of::<SidAttr>() {unsafe{LocalFree(admin)};return false}
 for i in 0..count {let entry=unsafe{read_unaligned(groups.as_ptr().add(offset+i*size_of::<SidAttr>()).cast::<SidAttr>())};if !entry.sid.is_null() && unsafe{EqualSid(entry.sid,admin)}!=0 && (entry.attributes&4!=0 || entry.attributes&0x10==0) {unsafe{LocalFree(admin)};return false}}
 unsafe{LocalFree(admin)};
 let mut privilege=vec![0u8;65536];if unsafe{GetTokenInformation(t,3,privilege.as_mut_ptr().cast(),privilege.len() as u32,&mut returned)}==0{return false}
 let count=unsafe{read_unaligned(privilege.as_ptr().cast::<u32>())} as usize;let offset=4;
 if count>(returned as usize).saturating_sub(offset)/size_of::<LuidAttributes>(){return false}
 let name:Vec<u16>="SeChangeNotifyPrivilege".encode_utf16().chain(Some(0)).collect();let mut allowed=Luid{low:0,high:0};if unsafe{LookupPrivilegeValueW(null(),name.as_ptr(),&mut allowed)}==0{return false}
 for i in 0..count {let item=unsafe{read_unaligned(privilege.as_ptr().add(offset+i*size_of::<LuidAttributes>()).cast::<LuidAttributes>())};if item.luid!=allowed{return false}}
 true
}
#[cfg(test_probe)] const WAIT_MS:u32=2000;
#[cfg(not(test_probe))] const WAIT_MS:u32=115000;
unsafe fn owned_by_token(path:&std::path::Path,t:Handle)->bool {
 let mut buf=vec![0u8;4096];let mut returned=0;
 if unsafe{GetTokenInformation(t,1,buf.as_mut_ptr().cast(),buf.len() as u32,&mut returned)}==0 || returned < size_of::<SidAttr>() as u32 {return false}
 let user=unsafe{read_unaligned(buf.as_ptr().cast::<SidAttr>())};if user.sid.is_null(){return false}
 let name=wide(path.as_os_str());let mut owner=null_mut();let mut descriptor=null_mut();
 let rc=unsafe{GetNamedSecurityInfoW(name.as_ptr(),1,1,&mut owner,null_mut(),null_mut(),null_mut(),&mut descriptor)};
 let ok=rc==0 && !owner.is_null() && unsafe{EqualSid(owner,user.sid)}!=0;
 if !descriptor.is_null(){unsafe{LocalFree(descriptor)};} ok
}
fn main(){unsafe{
 if std::env::args_os().len()!=1 {fail(50)}
 let data=std::path::PathBuf::from(std::env::var_os("NOTYPE_DATA_DIR").unwrap_or_else(||fail(51)));
 if !data.is_absolute() || data.file_name()!=Some(OsStr::new("notype-native-smoke")) {fail(52)}
 let base=data.parent().unwrap_or_else(||fail(53));
 let target=base.join("notype-assembled/extracted/app/node_modules/notype-win32-x64/app/notype.exe");
 let required=[data.clone(),data.join("profile/AppData/Roaming"),data.join("profile/AppData/Local"),data.join("temp"),target.clone()];
 for p in &required {let mut current=base.to_path_buf();let root_meta=std::fs::symlink_metadata(&current).unwrap_or_else(|_|fail(54));if root_meta.file_attributes()&0x400!=0 {fail(55)}
  for component in p.strip_prefix(base).unwrap_or_else(|_|fail(54)).components() {current.push(component);let m=std::fs::symlink_metadata(&current).unwrap_or_else(|_|fail(54));if m.file_attributes()&0x400!=0 {fail(55)}}}
 let target_abs=std::fs::canonicalize(&target).unwrap_or_else(|_|fail(56));
 let root_abs=std::fs::canonicalize(base).unwrap_or_else(|_|fail(57));
 if !target_abs.starts_with(&root_abs) || !std::fs::canonicalize(&data).unwrap_or_else(|_|fail(58)).starts_with(&root_abs) {fail(59)}
 let system=std::env::var_os("SystemRoot").unwrap_or_else(||fail(60));let system=std::path::PathBuf::from(system);if !system.is_absolute(){fail(61)}
 let profile=data.join("profile");let roam=profile.join("AppData/Roaming");let local=profile.join("AppData/Local");let temp=data.join("temp");
 let mut sys_path=system.join("System32").into_os_string();sys_path.push(";");sys_path.push(system.as_os_str());
 let mut vars=vec![("APPDATA",roam.into_os_string()),("LOCALAPPDATA",local.into_os_string()),("NOTYPE_DATA_DIR",data.clone().into_os_string()),("PATH",sys_path),("SystemRoot",system.clone().into_os_string()),("TEMP",temp.clone().into_os_string()),("TMP",temp.into_os_string()),("USERPROFILE",profile.into_os_string()),("WINDIR",system.into_os_string())];
 vars.sort_by_key(|(k,_)|k.to_ascii_uppercase());let mut env=Vec::<u16>::new();for (k,v) in vars {env.extend(OsStr::new(k).encode_wide());env.push(61);env.extend(v.encode_wide());env.push(0);}env.push(0);
 let mut original=null_mut();if OpenProcessToken(GetCurrentProcess(),0x8b,&mut original)==0{fail(1)}
 if !owned_by_token(&data,original) || !owned_by_token(&base.join("notype-assembled/extracted"),original) {fail(75)}
 let mut restricted=null_mut();if CreateRestrictedToken(original,5,0,null_mut(),0,null_mut(),0,null_mut(),&mut restricted)==0{fail(2)}
 let before=token_info(restricted);if before.2>8192 {let sidstr:Vec<u16>="S-1-16-8192".encode_utf16().chain(Some(0)).collect();let mut sid=null_mut();if ConvertStringSidToSidW(sidstr.as_ptr(),&mut sid)==0{fail(3)}let mut label=SidAttr{sid,attributes:0x20};let len=size_of::<SidAttr>() as u32+GetLengthSid(sid);if SetTokenInformation(restricted,25,(&mut label as *mut SidAttr).cast(),len)==0{fail(4)}LocalFree(sid);}
 if token_info(restricted)!=(0,0,8192){fail(5)}
 let job=CreateJobObjectW(null_mut(),null());if job.is_null(){fail(62)}let mut limits:ExtendedLimits=zeroed();limits.basic.flags=0x2000;
 if SetInformationJobObject(job,9,(&mut limits as *mut ExtendedLimits).cast(),size_of::<ExtendedLimits>() as u32)==0{fail(63)}
 let app=wide(target_abs.as_os_str());let mut command=vec![34u16];command.extend(target_abs.as_os_str().encode_wide());command.extend("\" --list-models".encode_utf16());command.push(0);
 let mut startup:StartupInfo=zeroed();startup.cb=size_of::<StartupInfo>() as u32;let mut pi:ProcessInfo=zeroed();
 if CreateProcessAsUserW(restricted,app.as_ptr(),command.as_mut_ptr(),null_mut(),null_mut(),0,0x08000404,env.as_mut_ptr().cast(),null(),&mut startup,&mut pi)==0{fail(7)}
 // On every post-creation failure terminate suspended child before closing handles.
 if AssignProcessToJobObject(job,pi.process)==0{if TerminateProcess(pi.process,1)==0 || WaitForSingleObject(pi.process,5000)!=0 {fail(74)} fail(64)}
 let mut child=null_mut();if OpenProcessToken(pi.process,8,&mut child)==0{TerminateJobObject(job,1);fail(65)}
 if token_info(child)!=(0,0,8192){TerminateJobObject(job,1);fail(66)}
 if !safe_token(child){TerminateJobObject(job,1);fail(69)}
 CloseHandle(child);
 if ResumeThread(pi.thread)==u32::MAX{TerminateJobObject(job,1);fail(70)}
 let wait=WaitForSingleObject(pi.process,WAIT_MS);if wait!=0 {TerminateJobObject(job,1);WaitForSingleObject(pi.process,5000);fail(71)}
 let mut exit=99;if GetExitCodeProcess(pi.process,&mut exit)==0{TerminateJobObject(job,1);fail(72)}
 CloseHandle(pi.thread);CloseHandle(pi.process);CloseHandle(job);CloseHandle(restricted);CloseHandle(original);
 if exit!=0 {eprintln!("error stage=73 win32={}",exit);std::process::exit(1)}
 println!("restricted smoke succeeded");
}}
