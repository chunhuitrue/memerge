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

// 可以多次peek_pkt。有一个独立的syn包
#[test]
fn test_peek_pkt_syn() {
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

    let ret_syn_pkt = stm.peek_pkt();
    assert_eq!(syn_pkt_seq, ret_syn_pkt.unwrap().seq());
    let ret_syn_pkt2 = stm.peek_pkt();
    assert_eq!(syn_pkt_seq, ret_syn_pkt2.unwrap().seq());    
    let ret_syn_pkt3 = stm.peek_pkt();
    assert_eq!(syn_pkt_seq, ret_syn_pkt3.unwrap().seq());    
}

// 可以多次peek_ord_pkt。有一个独立的syn包
#[test]
fn test_peek_ord_pkt_syn() {
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

    let ret_syn_pkt = stm.peek_ord_pkt();
    assert_eq!(syn_pkt_seq, ret_syn_pkt.unwrap().seq());
    let ret_syn_pkt2 = stm.peek_ord_pkt();
    assert_eq!(syn_pkt_seq, ret_syn_pkt2.unwrap().seq());    
    let ret_syn_pkt3 = stm.peek_ord_pkt();
    assert_eq!(syn_pkt_seq, ret_syn_pkt3.unwrap().seq());    
}

// pop_ord_data. syn包，3个数据，一个纯fin包
#[test]
fn test_pop_data_syn() {
    // syn 包seq占一个
    let syn_pkt_seq = 1;
    let syn_pkt = build_pkt_syn(syn_pkt_seq);
    let _ = syn_pkt.decode();
    // 2 - 11
    let seq1 = syn_pkt_seq + 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();
    // 12 - 21
    let seq2 = seq1 + pkt1.payload_len();
    let pkt2 = build_pkt(seq2, false);
    let _ = pkt2.decode();
    // 22 - 31
    let seq3 = seq2 + pkt2.payload_len();
    let pkt3 = build_pkt(seq3, false);
    let _ = pkt3.decode();
    // 32 无数据，fin
    let seq4 = seq3 + pkt3.payload_len();
    let pkt4 = build_pkt_fin(seq4);
    let _ = pkt4.decode();

    let mut stm = PktStrm::new();
    stm.push(syn_pkt);
    stm.push(pkt2);
    stm.push(pkt3);
    stm.push(pkt1);
    stm.push(pkt4);    

    let ret_syn_pkt = stm.peek_ord_pkt(); // peek ord pkt 可以看到syn包
    assert_eq!(syn_pkt_seq, ret_syn_pkt.unwrap().seq());
    let ret_syn_pkt2 = stm.peek_ord_pkt(); // 可以再次peek到syn包
    assert_eq!(syn_pkt_seq, ret_syn_pkt2.unwrap().seq());    
    
    let ret_pkt1 = stm.peek_ord_data();   // peek ord data 可以看到pkt1
    assert_eq!(seq1, ret_pkt1.unwrap().seq());
    
    let ret_pkt1 = stm.pop_ord_data();    // pop ord data 可以弹出pkt1
    assert_eq!(seq1, ret_pkt1.unwrap().seq());
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

// pop_ord_pkt. 独立的fin包。4个包乱序
#[test]
fn test_pop_ord_fin_4pkt() {
    // syn pkt
    let syn_seq = 0;
    let syn_pkt = build_pkt_syn(syn_seq);
    let _ = syn_pkt.decode();
    // 1 - 10
    let seq1 = syn_seq + 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();
    // 11 - 20
    let seq2 = seq1 + pkt1.payload_len();
    let pkt2 = build_pkt(seq2, false);
    let _ = pkt2.decode();
    // 21 - 30
    let seq3 = seq2 + pkt2.payload_len();
    let pkt3 = build_pkt(seq3, false);
    let _ = pkt3.decode();
    // 31 - 40
    let seq4 = seq3 + pkt3.payload_len();
    let pkt4 = build_pkt(seq4, false);
    let _ = pkt4.decode();
    // 41
    let fin_seq = seq4 + pkt3.payload_len();
    let fin_pkt = build_pkt_fin(fin_seq);
    let _ = fin_pkt.decode();

    let mut stm = PktStrm::new();
    stm.push(syn_pkt);    
    stm.push(pkt1);
    stm.push(pkt4);
    stm.push(pkt3);
    stm.push(pkt2);
    stm.push(fin_pkt);
    
    let ret_syn_pkt = stm.pop_ord_pkt();
    assert_eq!(syn_seq, ret_syn_pkt.clone().unwrap().seq());
    assert_eq!(0, ret_syn_pkt.unwrap().payload_len());    
    let ret_pkt1 = stm.pop_ord_pkt();
    assert_eq!(seq1, ret_pkt1.unwrap().seq());
    let ret_pkt2 = stm.pop_ord_pkt();
    assert_eq!(seq2, ret_pkt2.unwrap().seq());
    let ret_pkt3 = stm.pop_ord_pkt();
    assert_eq!(seq3, ret_pkt3.unwrap().seq());
    let ret_pkt4 = stm.pop_ord_pkt();
    assert_eq!(seq4, ret_pkt4.unwrap().seq());
    let ret_fin = stm.pop_ord_pkt();
    assert_eq!(fin_seq, ret_fin.clone().unwrap().seq());
    assert_eq!(0, ret_fin.unwrap().payload_len());
}
