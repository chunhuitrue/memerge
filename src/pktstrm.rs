#![allow(unused)]

use core::cmp::Ordering;
use std::cmp::Reverse;
use etherparse::TransportHeader;
use crate::Packet;
use std::collections::BinaryHeap;
use std::rc::Rc;
use crate::util::*;
use crate::Task;
use futures_util::{stream::{Stream, StreamExt}};
use std::{task::Poll};
use futures::future;

const MAX_CACHE_PKTS: usize = 32;

#[derive(Debug, Clone)]
pub struct PktStrm {
    cache: BinaryHeap<Reverse<SeqPacket>>,
    next_seq: u32,             // 下一个要读取的seq
    fin: bool
}

impl PktStrm {
    pub fn new() -> Self {
        PktStrm {
            cache: BinaryHeap::with_capacity(MAX_CACHE_PKTS),
            next_seq: 0,
            fin: false
        }
    }
    
    // 放入缓存，准备重组
    pub fn push(&mut self, pkt: Rc<Packet>) {
        if let Some(TransportHeader::Tcp(_)) = &pkt.header.borrow().as_ref().unwrap().transport {
            if self.cache.len() >= MAX_CACHE_PKTS {
                return;
            }
            
            self.cache.push(Reverse(SeqPacket(Rc::clone(&pkt))));
        }
    }

    // 无论是否严格seq连续，都peek一个当前包的clone
    fn peek(&self) -> Option<Rc<Packet>> {
        self.cache.peek().map(|rev_pkt| {
            let SeqPacket(pkt) = &rev_pkt.0;
            pkt.clone()
        })
    }

    // peek一个seq严格有序的包。如果当前top有序，就peek，否则就none
    fn peek_ord(&mut self) -> Option<Rc<Packet>> {
        if self.next_seq == 0 {
            if let Some(pkt) = self.peek() {
                self.next_seq = pkt.seq();                
            }
            return self.peek();
        }

        self.top_order();
        
        if let Some(pkt) = self.peek() {
            if pkt.seq() <= self.next_seq {
                return Some(pkt);
            }
        }
        None
    }

    // 清理掉top位置重复的包（但不是cache中所有重复的包）
    fn top_order(&mut self) {
        while let Some(pkt) = self.peek() {
            if pkt.seq() + pkt.payload_len() <= self.next_seq {
                self.pop_fin(pkt.clone());                
                continue;
            } else {
                return;
            }
        }
    }

    // 如果有序返回pkt，否则返回none
    // 同时会更新当前的seq。
    fn pop_ord(&mut self) -> Option<Rc<Packet>> {
        self.top_order();
        
        if let Some(pkt) = self.peek_ord() {
            if self.next_seq == 0 {
                self.next_seq = pkt.seq() + pkt.payload_len();
            } else if self.next_seq == pkt.seq() {
                self.next_seq += pkt.payload_len();                
            } else if self.next_seq > pkt.seq() {
                self.next_seq += pkt.payload_len() - (self.next_seq - pkt.seq());
            }
            
            self.pop_fin(pkt.clone());
            return Some(pkt);
        }
        None
    }

    fn pop_fin(&mut self, pkt: Rc<Packet>) {
        if pkt.fin() {
            self.fin = true;
        }
        self.pop();
    }
    
    // 无论是否严格seq连续，都pop一个当前包。
    // 注意：next_seq由调用者负责
    fn pop(&mut self) -> Option<Rc<Packet>> {
        self.cache.pop().map(|rev_pkt| rev_pkt.0.0)
    }
    
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    // 链接结束
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn timeout(&self) {
    }

    pub async fn readn(&mut self, num: usize) -> Vec<u8> {
        self.take(num).collect::<Vec<u8>>().await
    }

    pub async fn readline(&mut self) -> Vec<u8> {
        self.take_while(|x| future::ready(*x != b'\n')).collect::<Vec<u8>>().await
    }
}

impl Drop for PktStrm {
    fn drop(&mut self) {
        self.cache.clear();
    }
}

impl Default for PktStrm {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for PktStrm {
    type Item = u8;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        if let Some(pkt) = self.peek_ord() {
            let index = pkt.header.borrow().as_ref().unwrap().payload_offset as u32 + (self.next_seq - pkt.seq());
            if pkt.syn() && pkt.payload_len() == 0 {
                self.next_seq += 1;
                return Poll::Pending;                
            }
            if (index as usize) < pkt.data_len {
                self.next_seq += 1;
                return Poll::Ready(Some(pkt.data[index as usize])); 
            }
        } else if self.fin {
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}

#[derive(Debug, Clone)]
struct SeqPacket(Rc<Packet>);

impl PartialEq for SeqPacket {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0.header.borrow().as_ref().unwrap().transport, &other.0.header.borrow().as_ref().unwrap().transport) {
            (Some(TransportHeader::Tcp(s_tcph)), Some(TransportHeader::Tcp(o_tcph))) => {
                s_tcph.sequence_number == o_tcph.sequence_number
            }
            _ => false,
        }
    }
}

impl Eq for SeqPacket {}

impl PartialOrd for SeqPacket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SeqPacket {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0.header.borrow().as_ref().unwrap().transport, &other.0.header.borrow().as_ref().unwrap().transport) {
            (Some(TransportHeader::Tcp(s_tcph)), Some(TransportHeader::Tcp(o_tcph))) => {
                ntohl(s_tcph.sequence_number).cmp(&ntohl(o_tcph.sequence_number))
            }
            _ => Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use etherparse::*;    
    use crate::{ntohs, ntohl, htons, htonl};
    use core::{future::Future};    
    use crate::Task;
    use crate::TaskState;
    
    #[test]
    fn test_pkt() {
        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        assert_eq!(72, pkt1.data_len);
        assert_eq!(62, pkt1.header.borrow().as_ref().unwrap().payload_offset);
        assert_eq!(10, pkt1.header.borrow().as_ref().unwrap().payload_len);
        assert_eq!(25, pkt1.header.borrow().as_ref().unwrap().sport());
    }
    
    #[test]
    fn test_seqpacket_eq() {
        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        let pkt1 = SeqPacket(pkt1);
        let pkt2 = make_pkt_data(123);
        let _ = pkt2.decode();
        let pkt2 = SeqPacket(pkt2);
        assert_eq!(pkt1, pkt2);

        let pkt1 = make_pkt_data(123);        
        let _ = pkt1.decode();
        let pkt1 = SeqPacket(pkt1);
        let pkt2 = make_pkt_data(111);
        let _ = pkt2.decode();
        let pkt2 = SeqPacket(pkt2);
        assert_ne!(pkt1, pkt2);
        assert!(pkt1 > pkt2);
    }

    #[test]
    fn test_seqpacket_ord() {
        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        let pkt1 = SeqPacket(pkt1);
        let pkt2 = make_pkt_data(123);
        let _ = pkt2.decode();
        let pkt2 = SeqPacket(pkt2);
        assert!(pkt1 == pkt2);

        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        let pkt1 = SeqPacket(pkt1);
        let pkt2 = make_pkt_data(111);
        let _ = pkt2.decode();
        let pkt2 = SeqPacket(pkt2);
        assert!(pkt1 > pkt2);

        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        let pkt1 = SeqPacket(pkt1);
        let pkt2 = make_pkt_data(223);
        let _ = pkt2.decode();
        let pkt2 = SeqPacket(pkt2);
        assert!(pkt1 < pkt2);
    }

    #[test]
    fn test_push() {
        let mut stm = PktStrm::new();
        
        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        stm.push(pkt1);
        assert_eq!(1, stm.len());
        
        let pkt2 = make_pkt_data(123);
        let _ = pkt2.decode();
        stm.push(pkt2);
        assert_eq!(2, stm.len());
    }
    
    #[test]
    fn test_peek() {
        let mut stm = PktStrm::new();
        
        let pkt1 = make_pkt_data(1);
        let _ = pkt1.decode();
        stm.push(pkt1);

        let pkt2 = make_pkt_data(30);
        let _ = pkt2.decode();
        stm.push(pkt2);

        let pkt3 = make_pkt_data(80);
        let _ = pkt3.decode();
        stm.push(pkt3);
        
        assert_eq!(1, stm.peek().unwrap().seq());
        stm.pop();
        assert_eq!(30, stm.peek().unwrap().seq());
        stm.pop();
        assert_eq!(80, stm.peek().unwrap().seq());
        stm.pop();        
        assert!(stm.is_empty());
        stm.clear();        
    }

    // 插入的包严格有序 1-10 11-20 21-30
    #[test]
    fn test_peek_ord() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = make_pkt_data(seq1);
        let _ = pkt1.decode();
        // 11 - 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = make_pkt_data(seq2);
        let _ = pkt2.decode();
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = make_pkt_data(seq3);
        let _ = pkt3.decode();

        stm.push(pkt2.clone());
        stm.push(pkt3);
        stm.push(pkt1.clone());
        
        assert_eq!(seq1, stm.peek_ord().unwrap().seq());
        assert_eq!(seq1, stm.pop_ord().unwrap().seq());
        assert_eq!(seq2, stm.peek_ord().unwrap().seq());
        assert_eq!(seq2, stm.pop_ord().unwrap().seq());
        assert_eq!(seq3, stm.peek_ord().unwrap().seq());
        assert_eq!(seq3, stm.pop_ord().unwrap().seq());
        assert!(stm.is_empty());
        stm.clear();
    }

    // 插入的包有完整重传
    #[test]
    fn test_peek_ord_retrans() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = make_pkt_data(seq1);
        let _ = pkt1.decode();
        // 11- 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = make_pkt_data(seq2);
        let _ = pkt2.decode();
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = make_pkt_data(seq3);
        let _ = pkt3.decode();

        stm.push(pkt1.clone());        
        stm.push(pkt2.clone());
        stm.push(pkt1.clone());
        stm.push(pkt3.clone());

        assert_eq!(4, stm.len());
        assert_eq!(0, stm.next_seq);
        assert_eq!(seq1, stm.peek().unwrap().seq()); // 此时pkt1在top
        assert_eq!(seq1, stm.peek_ord().unwrap().seq());  // 看到pkt1
        assert_eq!(seq1, stm.pop_ord().unwrap().seq());   // 弹出pkt1, 通过pop_ord更新next_seq
        assert_eq!(pkt1.seq() + pkt1.payload_len(), stm.next_seq);
        
        assert_eq!(3, stm.len());                         // 此时重复的pkt1，仍在里面，top上
        assert_eq!(seq1, stm.peek().unwrap().seq());
        
        assert_eq!(seq2, stm.peek_ord().unwrap().seq()); // 看到pkt2
        assert_eq!(2, stm.len());         // peek_ord清理了重复的pkt1
        assert_eq!(pkt1.seq() + pkt1.payload_len(), stm.next_seq); //  peek_ord不会更新next_seq

        assert_eq!(seq2, stm.pop_ord().unwrap().seq());   // 弹出pkt2, 通过pop_ord更新next_seq
        assert_eq!(1, stm.len());
        assert_eq!(pkt1.seq() + pkt1.payload_len() + pkt2.payload_len(), stm.next_seq); //  peek_ord不会更新next_seq        
        
        assert_eq!(seq3, stm.peek().unwrap().seq()); // 此时pkt3在top
        assert_eq!(seq3, stm.peek_ord().unwrap().seq());  // 看到pkt3
        assert_eq!(seq3, stm.pop_ord().unwrap().seq());   // 弹出pkt3, 通过pop_ord更新next_seq
        assert_eq!(pkt1.seq() + pkt1.payload_len() + pkt2.payload_len() + pkt3.payload_len(), stm.next_seq);

        assert!(stm.is_empty());
        stm.clear();
    }

    // 插入的包有覆盖重传
    #[test]
    fn test_peek_ord_cover() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = make_pkt_data(seq1);
        let _ = pkt1.decode();
        // 11 - 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = make_pkt_data(seq2);
        let _ = pkt2.decode();
        // 15 - 24
        let seq3 = 15;
        let pkt3 = make_pkt_data(seq3);
        let _ = pkt3.decode();
        // 25 - 34
        let seq4 = 25;
        let pkt4 = make_pkt_data(seq4);
        let _ = pkt4.decode();

        stm.push(pkt1.clone());        
        stm.push(pkt2.clone());
        stm.push(pkt3.clone());
        stm.push(pkt4.clone());

        assert_eq!(4, stm.len());
        assert_eq!(0, stm.next_seq);
        
        assert_eq!(seq1, stm.peek().unwrap().seq()); // 此时pkt1在top
        assert_eq!(seq1, stm.peek_ord().unwrap().seq());  // 看到pkt1
        assert_eq!(seq1, stm.pop_ord().unwrap().seq());   // 弹出pkt1, 通过pop_ord更新next_seq
        assert_eq!(pkt1.seq() + pkt1.payload_len(), stm.next_seq);

        assert_eq!(3, stm.len());
        assert_eq!(seq2, stm.peek().unwrap().seq()); // 此时pkt2在top        
        assert_eq!(seq2, stm.pop_ord().unwrap().seq());   // 弹出pkt2, 通过pop_ord更新next_seq
        assert_eq!(seq2 + pkt2.payload_len(), stm.next_seq);
        
        assert_eq!(2, stm.len());
        assert_eq!(seq3, stm.peek().unwrap().seq()); // 此时pkt3在top
        assert_eq!(seq3, stm.pop_ord().unwrap().seq());   // 弹出pkt3, 通过pop_ord更新next_seq
        
        assert_eq!(seq3 + pkt3.payload_len(), stm.next_seq);
        assert_eq!(1, stm.len());        
        assert_eq!(seq4, stm.peek().unwrap().seq()); // 此时pkt4在top
        assert_eq!(seq4, stm.pop_ord().unwrap().seq());   // 弹出pkt4, 通过pop_ord更新next_seq

        assert_eq!(seq4 + pkt4.payload_len(), stm.next_seq);
        assert!(stm.is_empty());
        stm.clear();
    }

    #[test]    
    fn test_peek_drop() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = make_pkt_data(seq1);
        let _ = pkt1.decode();
        // 11- 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = make_pkt_data(seq2);
        let _ = pkt2.decode();
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = make_pkt_data(seq3);
        let _ = pkt3.decode();

        stm.push(pkt1.clone());        
        stm.push(pkt3.clone());
        
        assert_eq!(2, stm.len());
        assert_eq!(0, stm.next_seq);
        assert_eq!(seq1, stm.peek().unwrap().seq()); // 此时pkt1在top
        assert_eq!(seq1, stm.peek_ord().unwrap().seq());  // 看到pkt1
        assert_eq!(seq1, stm.pop_ord().unwrap().seq());   // 弹出pkt1, 通过pop_ord更新next_seq
        assert_eq!(pkt1.seq() + pkt1.payload_len(), stm.next_seq);

        assert_eq!(1, stm.len());
        assert_eq!(seq3, stm.peek().unwrap().seq()); // 此时pkt3在top
        assert_eq!(None, stm.peek_ord());  
        assert_eq!(None, stm.pop_ord());
        
        stm.clear();
    }

    // 插入的包严格有序 1-10 11-20 21-30, 最后一个带fin    
    #[test]    
    fn test_stm_fin() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = build_pkt(seq1, false);
        let _ = pkt1.decode();
        // 11 - 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = build_pkt(seq2, false);
        let _ = pkt2.decode();
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = build_pkt(seq3, true);
        let _ = pkt3.decode();

        stm.push(pkt2.clone());
        stm.push(pkt3);
        stm.push(pkt1.clone());

        assert_eq!(seq1, stm.pop_ord().unwrap().seq());
        assert!(!stm.fin);
        assert_eq!(seq2, stm.pop_ord().unwrap().seq());
        assert!(!stm.fin);
        assert_eq!(seq3, stm.pop_ord().unwrap().seq());
        assert!(stm.fin);
        assert!(stm.is_empty());        
        stm.clear();
    }

    async fn stream_task() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = build_pkt(seq1, true);
        let _ = pkt1.decode();

        stm.push(pkt1);

        for i in 0..10 {
            let ret = stm.next().await;
            println!("i:{}, ret:{}", i, ret.unwrap());
            assert_eq!(Some(i + 1), ret);
        }
        assert_eq!(None, stm.next().await);

        stm.clear();        
    }

    // 简单情况，一个包，带fin
    #[test]
    fn test_stream() {
        let mut task = Task::new(stream_task());
        assert_eq!(TaskState::Start, task.get_state());
        dbg!(task.get_state());
        task.run();
        assert_eq!(TaskState::End, task.get_state());
    }
    
    async fn stream_task_3pkt() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = build_pkt(seq1, false);
        let _ = pkt1.decode();
        // 11 - 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = build_pkt(seq2, false);
        let _ = pkt2.decode();
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = build_pkt(seq3, true);
        let _ = pkt3.decode();

        stm.push(pkt2.clone());
        stm.push(pkt3);
        stm.push(pkt1.clone());

        for j in 0..3 {
            for i in 0..10 {
                let ret = stm.next().await;
                println!("i:{}, ret:{}", i, ret.unwrap());
                assert_eq!(Some(i + 1), ret);
            }
        }
        assert_eq!(None, stm.next().await);

        stm.clear();        
    }

    // 三个包，最后一个包带fin
    #[test]
    fn test_stream_3pkt() {
        let mut task = Task::new(stream_task_3pkt());
        assert_eq!(TaskState::Start, task.get_state());
        task.run();
        assert_eq!(TaskState::End, task.get_state());
    }

    async fn stream_task_fin() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
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
        // 31 无数据，fin
        let seq4 = seq3 + pkt3.payload_len();
        let pkt4 = build_pkt_nodata(seq4, true);
        let _ = pkt4.decode();
        
        stm.push(pkt2.clone());
        stm.push(pkt3.clone());
        stm.push(pkt1.clone());
        stm.push(pkt4);        

        for j in 0..3 {
            for i in 0..10 {
                let ret = stm.next().await;
                println!("i:{}, ret:{}", i, ret.unwrap());
                assert_eq!(Some(i + 1), ret);
            }
        }
        assert_eq!(None, stm.next().await);

        stm.clear();
    }
    
    // 四个包，最后一个包只带fin
    #[test]
    fn test_stream_fin() {
        let mut task = Task::new(stream_task_fin());
        assert_eq!(TaskState::Start, task.get_state());
        task.run();
        assert_eq!(TaskState::End, task.get_state());
    }

    async fn stream_task_ack() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = build_pkt(seq1, false);
        let _ = pkt1.decode();
        // 11 - 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = build_pkt(seq2, false);
        let _ = pkt2.decode();
        // 21   ack 对方的seq 123，本包的seq是上一个包+1。载荷为0
        let ack_pkt_seq = seq2 + pkt2.payload_len();
        let ack_pkt = build_pkt_ack(ack_pkt_seq, 123);
        let _ = ack_pkt.decode();        
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = build_pkt(seq3, false);
        let _ = pkt3.decode();
        // 31 无数据，fin
        let seq4 = seq3 + pkt3.payload_len();
        let pkt4 = build_pkt_nodata(seq4, true);
        let _ = pkt4.decode();
        
        stm.push(pkt2.clone());
        stm.push(pkt3.clone());
        stm.push(ack_pkt);        
        stm.push(pkt1.clone());
        stm.push(pkt4);        

        for j in 0..3 {
            for i in 0..10 {
                let ret = stm.next().await;
                println!("i:{}, ret:{}", i, ret.unwrap());
                assert_eq!(Some(i + 1), ret);
            }
        }
        assert_eq!(None, stm.next().await);
        
        stm.clear();
    }

    // 中间有纯ack包的情况
    #[test]    
    fn test_stream_ack() {
        let mut task = Task::new(stream_task_ack());
        assert_eq!(TaskState::Start, task.get_state());
        task.run();
        assert_eq!(TaskState::End, task.get_state());
    }

    async fn stream_task_syn() {
        let mut stm = PktStrm::new();
        // syn 包seq占一个
        let syn_pkt_seq = 1;
        let syn_pkt = build_pkt_syn(syn_pkt_seq);
        let _ = syn_pkt.decode();
        // 1 - 10
        let seq1 = syn_pkt_seq + 1;
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
        // 31 无数据，fin
        let seq4 = seq3 + pkt3.payload_len();
        let pkt4 = build_pkt_nodata(seq4, true);
        let _ = pkt4.decode();

        stm.push(pkt4);
        stm.push(pkt2.clone());
        stm.push(pkt3.clone());
        stm.push(syn_pkt);
        stm.push(pkt1.clone());

        for j in 0..3 {
            for i in 0..10 {
                dbg!("up next");
                let ret = stm.next().await;
                println!("i:{}, ret:{}", i, ret.unwrap());
                assert_eq!(Some(i + 1), ret);
            }
        }
        assert_eq!(None, stm.next().await);
        
        stm.clear();
    }
    
    // syn包。同时也验证了中间中断，需要多次run的情况
    #[test]    
    fn test_stream_syn() {
        let mut task = Task::new(stream_task_syn());
        assert_eq!(TaskState::Start, task.get_state());
        task.run();             // 第一次run遇到第一个syn包，返回pending
        task.run();             // 第二次run到结束
        assert_eq!(TaskState::End, task.get_state());
    }

    async fn readn_task() {
        let mut stm = PktStrm::new();
        // syn 包seq占一个
        let syn_pkt_seq = 1;
        let syn_pkt = build_pkt_syn(syn_pkt_seq);
        let _ = syn_pkt.decode();
        // 1 - 10
        let seq1 = syn_pkt_seq + 1;
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
        // 31 无数据，fin
        let seq4 = seq3 + pkt3.payload_len();
        let pkt4 = build_pkt_nodata(seq4, true);
        let _ = pkt4.decode();

        stm.push(pkt4);
        stm.push(pkt2.clone());
        stm.push(pkt3.clone());
        stm.push(syn_pkt);
        stm.push(pkt1.clone());

        let res = stm.readn(5).await;
        assert_eq!(vec![1,2,3,4,5], res);
        let res = stm.readn(10).await;
        assert_eq!(vec![6,7,8,9,10,1,2,3,4,5], res);
        let res = stm.readn(15).await;
        assert_eq!(vec![6,7,8,9,10,1,2,3,4,5,6,7,8,9,10], res);
        let res = stm.readn(10).await;
        assert_eq!(Vec::<u8>::new(), res);
        
        stm.clear();        
    }
    
    // 4个包，还带syn，fin。看看是否可以跨包readn
    #[test]    
    fn test_readn() {
        let mut task = Task::new(readn_task());
        assert_eq!(TaskState::Start, task.get_state());
        task.run();             // 第一次run遇到第一个syn包，返回pending
        task.run();             // 第二次run到结束
        assert_eq!(TaskState::End, task.get_state());
    }    

    
    fn build_pkt(seq: u32, fin: bool) -> Rc<Packet> {
        //setup the packet headers
        let mut builder = PacketBuilder::
        ethernet2([1,2,3,4,5,6],     //source mac
                  [7,8,9,10,11,12]) //destionation mac
            .ipv4([192,168,1,1], //source ip
                  [192,168,1,2], //desitionation ip
                  20)            //time to life
            .tcp(25,    //source port 
                 htons(4000),  //desitnation port
                 htonl(seq),     //sequence number
                 1024) //window size
        //set additional tcp header fields
            .ns() //set the ns flag
        //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
            .ack(123) //ack flag + the ack number
            .urg(23) //urg flag + urgent pointer
            .options(&[
                TcpOptionElement::Noop,
                TcpOptionElement::MaximumSegmentSize(1234)
            ]).unwrap();
        if fin {
            builder = builder.fin();            
        }
        
        //payload of the tcp packet
        let payload = [1,2,3,4,5,6,7,8,9,10];
        //get some memory to store the result
        let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
        //serialize
        //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
        builder.write(&mut result, &payload).unwrap();
        // println!("result len:{}", result.len());
        
        Packet::new(1, result.len(), &result)
    }

    fn make_pkt_data(seq: u32) -> Rc<Packet> {
        build_pkt(seq, false)
    }

    fn build_pkt_nodata(seq: u32, fin: bool) -> Rc<Packet> {
        //setup the packet headers
        let mut builder = PacketBuilder::
        ethernet2([1,2,3,4,5,6],     //source mac
                  [7,8,9,10,11,12]) //destionation mac
            .ipv4([192,168,1,1], //source ip
                  [192,168,1,2], //desitionation ip
                  20)            //time to life
            .tcp(25,    //source port 
                 htons(4000),  //desitnation port
                 htonl(seq),     //sequence number
                 1024) //window size
        //set additional tcp header fields
            .ns() //set the ns flag
        //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
            .ack(123) //ack flag + the ack number
            .urg(23) //urg flag + urgent pointer
            .options(&[
                TcpOptionElement::Noop,
                TcpOptionElement::MaximumSegmentSize(1234)
            ]).unwrap();
        if fin {
            builder = builder.fin();            
        }
        
        //payload of the tcp packet
        let payload = [];
        //get some memory to store the result
        let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
        //serialize
        //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
        builder.write(&mut result, &payload).unwrap();
        // println!("result len:{}", result.len());
        
        Packet::new(1, result.len(), &result)
    }

    fn build_pkt_ack(seq: u32, ack_seq: u32) -> Rc<Packet> {
        //setup the packet headers
        let mut builder = PacketBuilder::
        ethernet2([1,2,3,4,5,6],     //source mac
                  [7,8,9,10,11,12]) //destionation mac
            .ipv4([192,168,1,1], //source ip
                  [192,168,1,2], //desitionation ip
                  20)            //time to life
            .tcp(25,    //source port 
                 htons(4000),  //desitnation port
                 htonl(seq),     //sequence number
                 1024) //window size
        //set additional tcp header fields
            .ns() //set the ns flag
        //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
            .ack(htonl(ack_seq)) //ack flag + the ack number
            .urg(23) //urg flag + urgent pointer
            .options(&[
                TcpOptionElement::Noop,
                TcpOptionElement::MaximumSegmentSize(1234)
            ]).unwrap();
        
        //payload of the tcp packet
        let payload = [];
        //get some memory to store the result
        let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
        //serialize
        //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
        builder.write(&mut result, &payload).unwrap();
        // println!("result len:{}", result.len());
        
        Packet::new(1, result.len(), &result)
    }

    fn build_pkt_syn(seq: u32) -> Rc<Packet> {
        //setup the packet headers
        let mut builder = PacketBuilder::
        ethernet2([1,2,3,4,5,6],     //source mac
                  [7,8,9,10,11,12]) //destionation mac
            .ipv4([192,168,1,1], //source ip
                  [192,168,1,2], //desitionation ip
                  20)            //time to life
            .tcp(25,    //source port 
                 htons(4000),  //desitnation port
                 htonl(seq),     //sequence number
                 1024) //window size
        //set additional tcp header fields
            .ns() //set the ns flag
        //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
            .syn()
            .urg(23) //urg flag + urgent pointer
            .options(&[
                TcpOptionElement::Noop,
                TcpOptionElement::MaximumSegmentSize(1234)
            ]).unwrap();
        
        //payload of the tcp packet
        let payload = [];
        //get some memory to store the result
        let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
        //serialize
        //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
        builder.write(&mut result, &payload).unwrap();
        // println!("result len:{}", result.len());
        
        Packet::new(1, result.len(), &result)
    }
}
