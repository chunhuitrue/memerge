use crate::common::*;

mod common;

#[test]
fn test_parser_smtp() {
    let pkt1 = make_pkt_data(123);
    let _ = pkt1.decode();
    assert_eq!(72, pkt1.data_len);
    assert_eq!(62, pkt1.header.borrow().as_ref().unwrap().payload_offset);
    assert_eq!(10, pkt1.header.borrow().as_ref().unwrap().payload_len);
    assert_eq!(25, pkt1.header.borrow().as_ref().unwrap().sport());
}
