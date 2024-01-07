use core::cmp::Ordering;
use std::cmp::Reverse;
use etherparse::TransportHeader;
use crate::Packet;
use std::collections::BinaryHeap;
use std::rc::Rc;

const MAX_CACHE_PKTS: usize = 32;

#[derive(Debug, Clone)]
pub struct PktStrm {
    cache: BinaryHeap<Reverse<SeqPacket>>,
    expect_seq: u32
}

impl PktStrm {
    pub fn new() -> Self {
        PktStrm { cache: BinaryHeap::with_capacity(MAX_CACHE_PKTS),
                  expect_seq: 0
        }
    }

    /// 放入缓存，准备重组
    ///
    pub fn put(&mut self, pkt: Rc<Packet>) {
        if let Some(TransportHeader::Tcp(_)) = &pkt.header.borrow().as_ref().unwrap().transport {
            if self.cache.len() >= MAX_CACHE_PKTS {
                return;
            }
            
            self.cache.push(Reverse(SeqPacket(Rc::clone(&pkt))));
        }
    }

    /// 得到一个严格有序，连续seq的数据包
    pub fn get_ord_pkt(&mut self) -> Option<Rc<Packet>> {
        if self.expect_seq == 0 {
            if let Some(pkt) = self.cache.pop().map(|rev_pkt| rev_pkt.0.0) {
                self.expect_seq = pkt.seq() + pkt.data_len;
                return Some(pkt);
            } else {
                return None;
            }
        }

        if let Some(pkt) = self.cache.peek().map(|rev_pkt| &rev_pkt.0.0) {
            if pkt.seq() == self.expect_seq {
                self.expect_seq += pkt.data_len;
                return self.cache.pop().map(|rev_pkt| rev_pkt.0.0);
            } else if pkt.seq() + pkt.data_len <= self.expect_seq {
                self.cache.pop();
                return None;
            } else if pkt.seq() < self.expect_seq && pkt.seq() + pkt.data_len > self.expect_seq {
                self.expect_seq =  pkt.seq() + pkt.data_len;
                return self.cache.pop().map(|rev_pkt| rev_pkt.0.0);                
            }
            None
        } else {
            None
        }
    }
    
    /// 链接结束
    pub fn finish(&mut self) {
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
                s_tcph.sequence_number.cmp(&o_tcph.sequence_number)
            }
            _ => Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn tt() {
        
    // }
}
