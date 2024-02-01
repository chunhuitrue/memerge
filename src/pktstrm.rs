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

const MAX_CACHE_PKTS: usize = 32;

#[derive(Debug, Clone)]
pub struct PktStrm {
    cache: BinaryHeap<Reverse<SeqPacket>>,
    next_seq: u32             // 下一个要读取的seq
}

impl PktStrm {
    pub fn new() -> Self {
        PktStrm {
            cache: BinaryHeap::with_capacity(MAX_CACHE_PKTS),
            next_seq: 0
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
                self.pop();
                continue;
            } else {
                return;
            }
        }
    }
    
    // 无论是否严格seq连续，都pop一个当前包。
    // 注意：next_seq由调用者负责
    fn pop(&mut self) -> Option<Rc<Packet>> {
        self.cache.pop().map(|rev_pkt| rev_pkt.0.0)
    }

    // 如果有序返回pkt，否则返回none
    // 同时会更新当前的seq。
    pub fn pop_ord(&mut self) -> Option<Rc<Packet>> {
        self.top_order();
        
        if let Some(pkt) = self.peek_ord() {
            if self.next_seq == 0 {
                self.next_seq = pkt.seq() + pkt.payload_len();
            } else if self.next_seq == pkt.seq() {
                self.next_seq += pkt.payload_len();                
            } else if self.next_seq > pkt.seq() {
                self.next_seq += pkt.payload_len() - (self.next_seq - pkt.seq());
            }
            
            self.pop();
            return Some(pkt);
        }
        None
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
            let index = self.next_seq - pkt.seq();
            self.next_seq += 1;
            return Poll::Ready(Some(pkt.data[index as usize])); 
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
    
    fn make_pkt_data(seq: u32) -> Rc<Packet> {
        //setup the packet headers
        let builder = PacketBuilder::
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
}
