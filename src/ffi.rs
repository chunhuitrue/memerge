extern crate libc;
use std::ptr;
use crate::{Task, PktDirection, Packet, Meta, smtp::{SmtpParser, MetaSmtp}};
use std::ffi::{CString, c_char};

#[repr(C)] #[allow(dead_code)]
pub enum ParserType {
    Smtp,
    Http,
    Undef
}

#[repr(C)]
pub enum PacketDir {
    C2s,
    S2c,
    BiDir,
    Unknown
}

impl From<PacketDir> for PktDirection {
    fn from(dir: PacketDir) -> PktDirection {
        match dir {
            PacketDir::C2s => PktDirection::Client2Server,
            PacketDir::S2c => PktDirection::Server2Client,
            PacketDir::BiDir => PktDirection::BiDirection,
            PacketDir::Unknown => PktDirection::Unknown,
        }
    }
}

#[no_mangle]
pub extern fn task_new() -> *mut Task {
    Box::into_raw(Box::new(Task::new()))
}

#[no_mangle]
pub extern fn task_free(ptr: *mut Task) {
    if ptr.is_null()  {
        return;
    }

    unsafe { let _ = Box::from_raw(ptr); }
}

#[no_mangle]
pub extern fn task_new_with_parser(parser_type: ParserType) -> *mut Task{
    match parser_type {
        ParserType::Smtp => {
            Box::into_raw(Box::new(Task::new_with_parser(SmtpParser)))
        }
        _ => {
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern fn task_init_parser(task_ptr: *mut Task, parser_type: ParserType) -> *mut Task {
    if task_ptr.is_null()  {
        return task_ptr;
    }

    let task = unsafe { &mut *task_ptr }; 
    match parser_type {
        ParserType::Smtp => {
            task.init_parser(SmtpParser);
            task_ptr
        }
        _ => {        
            task_ptr
        }
    }
}

#[no_mangle]
pub extern fn task_run(task_ptr: *mut Task, pkt: *const u8, pkt_len: usize, pkt_dir: PacketDir, ts: u64) {
    if task_ptr.is_null() || pkt.is_null() {
        return;
    }

    let task = unsafe { &mut *task_ptr };     
    let data = unsafe { std::slice::from_raw_parts(pkt, pkt_len) };
    let packet = Packet::new(ts.into(), pkt_len, data);
    if packet.decode().is_err() {
        return;
    }

    task.run(packet, pkt_dir.into());
}

#[no_mangle]
pub extern fn task_get_meta(task_ptr: *mut Task) -> *const Meta {
    if task_ptr.is_null() {
        return ptr::null_mut();
    }

    let task = unsafe { &mut *task_ptr }; 
    if let Some(meta) = task.get_meta() {
        return Box::into_raw(Box::new(meta));
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern fn meta_free(meta_ptr: *mut Meta) {
    if meta_ptr.is_null() {
        return;
    }

    unsafe { let _ = Box::from_raw(meta_ptr); }
}

#[no_mangle]
pub extern fn meta_protocol(meta_ptr: *mut Meta) -> ParserType {
    if meta_ptr.is_null() {
        return ParserType::Undef;
    }

    let meta = unsafe { &*meta_ptr };    
    match *meta {
        Meta::Smtp(_) => {
            ParserType::Smtp
        }
        Meta::Http(_) => {
            ParserType::Http
        }
    }
}

#[repr(C)] #[allow(dead_code)]
pub enum MetaSmtpType {
    User,
    Pass,
    MailFrom,
    RcptTo,
    Subject,
    None,
}

#[no_mangle]
pub extern fn smtp_meta_type(meta_ptr: *mut Meta) -> MetaSmtpType {
    if meta_ptr.is_null() {
        return MetaSmtpType::None;
    }

    let meta = unsafe { &*meta_ptr };
    match meta {
        Meta::Smtp(smtp) => {
            match smtp {
                MetaSmtp::User(_) => MetaSmtpType::User,
                MetaSmtp::Pass(_) => MetaSmtpType::Pass,
                MetaSmtp::MailFrom(_, _) => MetaSmtpType::MailFrom,
                MetaSmtp::RcptTo(_) => MetaSmtpType::RcptTo,
                MetaSmtp::Subject(_) => MetaSmtpType::Subject,
            }
        }
        _ => MetaSmtpType::None,
    }
}

#[no_mangle]
pub extern fn smtp_meta_user(meta_ptr: *mut Meta) -> *const c_char{
    if meta_ptr.is_null() {
        return ptr::null();
    }

    let meta = unsafe { &*meta_ptr };
    if let Meta::Smtp(MetaSmtp::User(ref user)) = *meta {
        let c_string = CString::new(user.as_str()).unwrap();
        return c_string.into_raw();
    }
    ptr::null()
}

#[no_mangle]
pub extern fn smtp_meta_user_free(user: *mut c_char) {
    if user.is_null() {
        return;
    }

    unsafe { let _ = CString::from_raw(user); }
}
