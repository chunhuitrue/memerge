extern crate libc;

#[no_mangle]
pub extern fn addition(a: u32, b: u32) -> u32 {
    a + b
}
