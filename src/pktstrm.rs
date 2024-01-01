use core::cmp::Ordering;
use etherparse::TransportHeader;
use packet::Packet;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct PktStrm<'a> {
    pkt_cache: BinaryHeap<SeqPacket<'a>>,
}

impl<'a> PktStrm<'a> {
    pub fn new() -> Self {
        PktStrm { pkt_cache: BinaryHeap::new() }
    }

    /// 数据包处理，放入缓存
    pub fn put(&mut self, _pkt: &Packet) {
        // 需要判断是否是tcp
        todo!()
    }

    /// 链接结束
    pub fn finish(&mut self) {
        todo!()
    }

    pub fn timeout(&self) {
    }
}

impl Default for PktStrm<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct SeqPacket<'a>(Rc<Packet<'a>>);

impl PartialEq for SeqPacket<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0.header.borrow().as_ref().unwrap().transport, &other.0.header.borrow().as_ref().unwrap().transport) {
            (Some(TransportHeader::Tcp(s_tcph)), Some(TransportHeader::Tcp(o_tcph))) => {
                s_tcph.sequence_number == o_tcph.sequence_number
            }
            _ => false,
        }
    }
}

impl Eq for SeqPacket<'_> {}

impl PartialOrd for SeqPacket<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SeqPacket<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0.header.borrow().as_ref().unwrap().transport, &other.0.header.borrow().as_ref().unwrap().transport) {
            (Some(TransportHeader::Tcp(s_tcph)), Some(TransportHeader::Tcp(o_tcph))) => {
                s_tcph.sequence_number.cmp(&o_tcph.sequence_number)
            }
            _ => Ordering::Equal,
        }
    }
}
