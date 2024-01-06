use core::cmp::Ordering;
use etherparse::TransportHeader;
use crate::Packet;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct PktStrm {
    pkt_cache: BinaryHeap<SeqPacket>,
}

impl PktStrm {
    pub fn new() -> Self {
        PktStrm { pkt_cache: BinaryHeap::new() }
    }

    /// 数据包处理，放入缓存
    pub fn put(&mut self, pkt: &Packet) {
        
        // 需要判断是否是tcp
        todo!()
    }

    /// 链接结束
    pub fn finish(&mut self) {
    }

    pub fn timeout(&self) {
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
