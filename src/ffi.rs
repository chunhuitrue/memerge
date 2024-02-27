extern crate libc;
use crate::Task;
use crate::parser;
use crate::smtp;

#[repr(C)]
pub enum ParserType {
    SmtpParse,
}

#[no_mangle]
pub extern fn task_new() -> *mut Task {
    Box::into_raw(Box::new(Task::new()))
}

#[no_mangle]
pub extern fn task_new_with_parser(parser_type: ParserType) {
    match parser_type {
        ParserType::SmtpParse => {
            
        }
        _ => {}
    }
}

#[no_mangle]
pub extern fn task_free(ptr: *mut Task) {
    if ptr.is_null()  {
        return;
    }

    unsafe { let _ = Box::from_raw(ptr); }
}





#[no_mangle]
pub extern fn addition(a: u32, b: u32) -> u32 {
    a + b
}

