extern crate libc;
use std::ptr;
use crate::Task;
use crate::smtp::SmtpParser;

#[repr(C)] #[allow(dead_code)]
pub enum ParserType {
    Smtp,
    Http,
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
pub extern fn init_parser(ptr: *mut Task, parser_type: ParserType) -> *mut Task {
    if ptr.is_null()  {
        return ptr;
    }

    unsafe {
        let mut task = Box::from_raw(ptr);
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
}

// todo task run

#[no_mangle]
pub extern fn task_free(ptr: *mut Task) {
    if ptr.is_null()  {
        return;
    }

    unsafe { let _ = Box::from_raw(ptr); }
}



