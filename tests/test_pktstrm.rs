mod common;

use crate::common::*;
use memerge::*;

// pop_ord_pkt. 一个syn，一个正常包。
#[test]
fn test_pop_ord_syn() {
    // syn 包seq占一个
    let syn_pkt_seq = 1;
    let syn_pkt = build_pkt_syn(syn_pkt_seq);
    let _ = syn_pkt.decode();
    // 2 - 11
    let seq1 = syn_pkt_seq + 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();

    let mut stm = PktStrm::new();
    stm.push(syn_pkt);
    stm.push(pkt1);

    let ret_syn_pkt = stm.pop_ord_pkt();
    assert_eq!(1, ret_syn_pkt.unwrap().seq());
    let ret_pkt1 = stm.pop_ord_pkt();
    assert_eq!(2, ret_pkt1.unwrap().seq());
}

// pop_ord_pkt. syn包从0开始
#[test]
fn test_pop_ord_syn_seq0() {
    // syn 包seq占一个
    let syn_pkt_seq = 0;
    let syn_pkt = build_pkt_syn(syn_pkt_seq);
    let _ = syn_pkt.decode();
    // 1 - 10
    let seq1 = syn_pkt_seq + 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();

    let mut stm = PktStrm::new();
    stm.push(syn_pkt);
    stm.push(pkt1);

    let ret_syn_pkt = stm.pop_ord_pkt();
    assert_eq!(0, ret_syn_pkt.unwrap().seq());
    let ret_pkt1 = stm.pop_ord_pkt();
    assert_eq!(1, ret_pkt1.unwrap().seq());
}

// pop_ord_pkt. 独立的fin包
#[test]
fn test_pop_ord_fin() {
    // 1 - 10
    let seq1 = 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();
    // 11 - 20
    let seq2 = seq1 + pkt1.payload_len();
    let pkt2 = build_pkt_fin(seq2);
    let _ = pkt2.decode();

    let mut stm = PktStrm::new();
    stm.push(pkt1);
    stm.push(pkt2);
    
    let ret_pkt1 = stm.pop_ord_pkt();
    assert_eq!(1, ret_pkt1.unwrap().seq());
    let ret_pkt2 = stm.pop_ord_pkt();
    assert_eq!(11, ret_pkt2.unwrap().seq());
}

