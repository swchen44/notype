//! Directed clipboard delivery. Original formats remain opaque and are never logged.
//! A timed-out paste stays uncertain: its worker restores only after the remote handler ends.
use notype_core::DeliveryOutcome;
use std::{sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU8,Ordering},mpsc},time::Duration};
static DELIVERY:Mutex<()>=Mutex::new(());

pub fn paste(control:isize,text:String,before:impl Fn()->bool+Send+'static,after:impl Fn()->bool+Send+'static)->DeliveryOutcome {
    let (send,receive)=mpsc::sync_channel(1);
    // 0=not sent, 1=write committed, 2=cancelled before send.
    let state=Arc::new(AtomicU8::new(0));let worker_state=state.clone();
    if std::thread::Builder::new().name("notype-directed-paste".into()).spawn(move||{
        let result=run(control,&text,&before,&after,&worker_state);
        let _=send.send(result);
    }).is_err(){return DeliveryOutcome::Unavailable;}
    receive.recv_timeout(Duration::from_secs(2)).unwrap_or_else(|_|{
        if state.compare_exchange(0,2,Ordering::SeqCst,Ordering::SeqCst).is_ok(){DeliveryOutcome::Unavailable}else{DeliveryOutcome::Uncertain}
    })
}
fn run(control:isize,text:&str,before:&dyn Fn()->bool,after:&dyn Fn()->bool,state:&AtomicU8)->DeliveryOutcome {
    let Ok(_lease)=DELIVERY.try_lock() else{return DeliveryOutcome::Unavailable;};
    if !before() || state.load(Ordering::SeqCst)==2{return DeliveryOutcome::TargetChanged;}
    let Some(mut clipboard)=Clipboard::publish(text) else{return DeliveryOutcome::Unavailable;};
    if !before() || state.compare_exchange(0,1,Ordering::SeqCst,Ordering::SeqCst).is_err(){return DeliveryOutcome::TargetChanged;}
    let done=Arc::new(AtomicBool::new(false));let callback_ref=Arc::into_raw(done.clone());
    clipboard.safe_to_restore=false;
    let sent=unsafe{SendMessageCallbackW(control,0x0302,0,0,Some(completed),callback_ref as usize)};
    if sent==0 {
        unsafe{drop(Arc::from_raw(callback_ref));} clipboard.safe_to_restore=true;
        return DeliveryOutcome::Unavailable;
    }
    while !done.load(Ordering::Acquire) {
        let mut message:Message=unsafe{std::mem::zeroed()};
        unsafe{while PeekMessageW(&mut message,0,0,0,1)!=0 {DispatchMessageW(&message);}}
        // A destroyed original window cannot later receive this directed paste.
        if unsafe{IsWindow(control)}==0 {break;}
        std::thread::sleep(Duration::from_millis(5));
    }
    clipboard.safe_to_restore=true;
    let delivered=done.load(Ordering::Acquire) && after();
    let restored=clipboard.restore();
    if delivered && restored {DeliveryOutcome::Delivered}else{DeliveryOutcome::Uncertain}
}
unsafe extern "system" fn completed(_:isize,_:u32,data:usize,_:isize){
    let done=unsafe{Arc::from_raw(data as *const AtomicBool)};done.store(true,Ordering::Release);
}
struct Format { kind:u32, handle:isize }
impl Drop for Format {
    fn drop(&mut self){unsafe{if self.handle!=0 {match self.kind {2|9|0x82=>{DeleteObject(self.handle);},14|0x8e=>{DeleteEnhMetaFile(self.handle);},_=>{GlobalFree(self.handle);}}}}}
}
struct Clipboard { owner:isize, original:Vec<Format>, sequence:u32, safe_to_restore:bool, restored:bool }
impl Clipboard {
    fn publish(text:&str)->Option<Self>{unsafe{
        let class=[83u16,84,65,84,73,67,0];
        let owner=CreateWindowExW(0,class.as_ptr(),std::ptr::null(),0,0,0,0,0,-3,0,0,std::ptr::null());
        if owner==0{return None;}
        let mut guard=Self{owner,original:vec![],sequence:0,safe_to_restore:true,restored:true};
        if OpenClipboard(owner)==0{return None;}
        let snapshot=(||{
            let mut format=0;let mut size=0usize;
            loop {
                format=EnumClipboardFormats(format);if format==0{break;}
                if guard.original.len()>=256 || matches!(format,3|0x80|0x83|0x300..=0x3ff){return None;}
                let handle=GetClipboardData(format);if handle==0{return None;}
                let bytes=match format {
                    2|0x82=>{let mut bitmap:Bitmap=std::mem::zeroed();if GetObjectW(handle,std::mem::size_of::<Bitmap>() as i32,&mut bitmap as *mut _ as *mut _)==0{return None;} (bitmap.width_bytes.unsigned_abs() as usize).checked_mul(bitmap.height.unsigned_abs() as usize)?},
                    14|0x8e=>GetEnhMetaFileBits(handle,0,std::ptr::null_mut()) as usize,
                    9=>256*1024,
                    _=>{let size=GlobalSize(handle);if size==0{return None;}size},
                };
                size=size.checked_add(bytes)?;if size>64*1024*1024{return None;}
                let copy=OleDuplicateData(handle,format,0);if copy==0{return None;}
                guard.original.push(Format{kind:format,handle:copy});
            }
            Some(())
        })();
        if snapshot.is_none(){CloseClipboard();return None;}
        let mut wide:Vec<u16>=text.encode_utf16().collect();wide.push(0);
        let memory=GlobalAlloc(0x42,wide.len()*2);
        if memory==0{CloseClipboard();return None;}
        let destination=GlobalLock(memory);
        if destination.is_null(){GlobalFree(memory);CloseClipboard();return None;}
        std::ptr::copy_nonoverlapping(wide.as_ptr(),destination as *mut u16,wide.len());GlobalUnlock(memory);
        if EmptyClipboard()==0{GlobalFree(memory);CloseClipboard();return None;}
        guard.restored=false;
        if SetClipboardData(13,memory)==0{GlobalFree(memory);guard.sequence=GetClipboardSequenceNumber();CloseClipboard();return None;}
        guard.sequence=GetClipboardSequenceNumber();CloseClipboard();Some(guard)
    }}
    fn restore(&mut self)->bool{unsafe{
        if self.restored{return true;}
        if !self.safe_to_restore{return false;}
        if OpenClipboard(self.owner)==0{return false;}
        // A newer user copy takes precedence; never overwrite it with our backup.
        if GetClipboardSequenceNumber()!=self.sequence{CloseClipboard();self.restored=true;return true;}
        if EmptyClipboard()==0{CloseClipboard();return false;}
        let mut success=true;
        for format in &mut self.original {
            if SetClipboardData(format.kind,format.handle)==0{success=false;}else{format.handle=0;}
        }
        CloseClipboard();self.restored=success;success
    }}
}
impl Drop for Clipboard {fn drop(&mut self){if self.safe_to_restore{let _=self.restore();}unsafe{DestroyWindow(self.owner);}}}
#[repr(C)]struct Point{x:i32,y:i32}
#[repr(C)]struct Message{window:isize,id:u32,w:usize,l:isize,time:u32,point:Point,private:u32}
#[repr(C)]struct Bitmap{kind:i32,width:i32,height:i32,width_bytes:i32,planes:u16,bits_pixel:u16,bits:*mut std::ffi::c_void}
#[link(name="user32")]unsafe extern "system"{
    fn CreateWindowExW(ex:u32,class:*const u16,name:*const u16,style:u32,x:i32,y:i32,w:i32,h:i32,parent:isize,menu:isize,instance:isize,param:*const std::ffi::c_void)->isize;
    fn DestroyWindow(window:isize)->i32;fn IsWindow(window:isize)->i32;
    fn OpenClipboard(owner:isize)->i32;fn CloseClipboard()->i32;fn EmptyClipboard()->i32;
    fn EnumClipboardFormats(format:u32)->u32;fn GetClipboardData(format:u32)->isize;
    fn SetClipboardData(format:u32,data:isize)->isize;fn GetClipboardSequenceNumber()->u32;
    fn SendMessageCallbackW(window:isize,id:u32,w:usize,l:isize,callback:Option<unsafe extern "system"fn(isize,u32,usize,isize)>,data:usize)->i32;
    fn PeekMessageW(message:*mut Message,window:isize,min:u32,max:u32,remove:u32)->i32;fn DispatchMessageW(message:*const Message)->isize;
}
#[link(name="kernel32")]unsafe extern "system"{
    fn GlobalAlloc(flags:u32,bytes:usize)->isize;fn GlobalLock(memory:isize)->*mut std::ffi::c_void;
    fn GlobalUnlock(memory:isize)->i32;fn GlobalFree(memory:isize)->isize;fn GlobalSize(memory:isize)->usize;
}
#[link(name="ole32")]unsafe extern "system"{fn OleDuplicateData(data:isize,format:u32,flags:u32)->isize;}
#[link(name="gdi32")]unsafe extern "system"{
    fn DeleteObject(object:isize)->i32;fn DeleteEnhMetaFile(object:isize)->i32;
    fn GetObjectW(object:isize,size:i32,data:*mut std::ffi::c_void)->i32;fn GetEnhMetaFileBits(handle:isize,size:u32,data:*mut u8)->u32;
}
