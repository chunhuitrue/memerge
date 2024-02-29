extern crate libc;
use std::ptr;
use crate::{Task, PktDirection, Packet};
use crate::smtp::SmtpParser;

#[repr(C)] #[allow(dead_code)]
pub enum ParserType {
    Smtp,
    Http,
}

#[repr(C)] #[allow(dead_code)]
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
pub extern fn task_new_with_parser(parser_type: ParserType) -> *mut Task{
    match parser_type {
        ParserType::Smtp => {
            Box::into_raw(Box::new(Task::new_with_parser(SmtpParser)))
        }
        ParserType::Http => {
            ptr::null_mut()
        }
    }
}


#[no_mangle]
pub extern fn task_init_parser(ptr: *mut Task, parser_type: ParserType) -> *mut Task {
    if ptr.is_null()  {
        return ptr;
    }

    let mut task;
    unsafe { task = Box::from_raw(ptr); }
    match parser_type {
        ParserType::Smtp => {
            task.init_parser(SmtpParser);
            Box::into_raw(task)
        }
        ParserType::Http => {
            Box::into_raw(task)
        }
    }
}

#[no_mangle]
pub extern fn task_run(task_ptr: *mut Task, pkt: *const u8, pkt_len: usize, pkt_dir: PacketDir, ts: u64) {
    if task_ptr.is_null() || pkt.is_null() {
        return;
    }

    let mut task;
    unsafe { task = Box::from_raw(task_ptr); }
    let data = unsafe { std::slice::from_raw_parts(pkt, pkt_len) };
    let packet = Packet::new(ts.into(), pkt_len, data);
    if packet.decode().is_err() {
        Box::into_raw(task);
        return;
    }

    task.run(packet, pkt_dir.into());
    Box::into_raw(task);    
}

#[no_mangle]
pub extern fn task_free(ptr: *mut Task) {
    if ptr.is_null()  {
        return;
    }

    unsafe { let _ = Box::from_raw(ptr); }
}



