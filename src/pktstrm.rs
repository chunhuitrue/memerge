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
    pub fn peek(&self) -> Option<Rc<Packet>> {
        self.cache.peek().map(|rev_pkt| {
            let SeqPacket(pkt) = &rev_pkt.0;
            pkt.clone()
        })
    }

    // peek一个seq严格有序的包，有重复，就pop。直到不有序为止
    pub fn peek_ord(&mut self) -> Option<Rc<Packet>> {
        if self.expect_seq == 0 {
           return self.peek();
        }
        
        while let Some(pkt) = self.peek() {
            if pkt.seq() == self.expect_seq {
                return self.peek();
            } else if pkt.seq() + pkt.payload_len() <= self.expect_seq {
                self.pop();
                continue;
            } else if pkt.seq() < self.expect_seq && pkt.seq() + pkt.payload_len() > self.expect_seq {
                return self.peek();                
            } else {
                return None;
            }
        }
        None
    }
    
    // // peek一个seq严格有序的包，不严格有序就为None
    // pub fn peek_ord(&self) -> Option<Rc<Packet>> {
    //     if self.expect_seq == 0 {
    //        return self.peek();
    //     }

    //     if let Some(pkt) = self.peek() {
    //         if pkt.seq() == self.expect_seq {
    //             return self.peek();
    //         } else if pkt.seq() + pkt.payload_len() <= self.expect_seq {
    //             return None;
    //         } else if pkt.seq() < self.expect_seq && pkt.seq() + pkt.payload_len() > self.expect_seq {
    //             return self.peek();
    //         }
    //         None
    //     } else {
    //         None
    //     }
    // }

    // // 判断当前包是否是seq严格有序
    // pub fn is_top_ord(&self) -> bool {
    //     if self.peek_ord().is_none() {
    //         return false;
    //     }
    //     true
    // }
    
    // 无论是否严格seq连续，都pop一个当前包
    pub fn pop(&mut self) -> Option<Rc<Packet>> {
        self.cache.pop().map(|rev_pkt| rev_pkt.0.0)
    }

    // // pop一个seq严格有序的包,如果top的不是有序，None
    // pub fn pop_ord(&mut self) -> Option<Rc<Packet>> {
    //     if let Some(pkt) = self.peek_ord() {
    //         if self.expect_seq == 0 {
    //             self.expect_seq = pkt.seq() + pkt.payload_len();
    //             return self.pop();
    //         }

    //         if pkt.seq() == self.expect_seq {
    //             self.expect_seq += pkt.payload_len();
    //             self.pop()
    //         } else if pkt.seq() + pkt.payload_len() <= self.expect_seq {
    //             self.pop();
    //             return None;
    //         } else if pkt.seq() < self.expect_seq && pkt.seq() + pkt.payload_len() > self.expect_seq {
    //             self.expect_seq = pkt.seq() + pkt.payload_len();
    //             return self.pop();                
    //         } else {
    //             return None;
    //         }
    //     } else {
    //         None
    //     }
    // }

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
    use super::*;
    use etherparse::*;    

    #[test]
    fn test_pkt() {
        let pkt1 = make_pkt_data(123);
        let _ = pkt1.decode();
        dbg!(pkt1.data_len);        
        dbg!(pkt1.header.borrow().as_ref().unwrap().payload_offset);
        dbg!(pkt1.header.borrow().as_ref().unwrap().payload_len);
        assert_eq!(72, pkt1.data_len);
        assert_eq!(62, pkt1.header.borrow().as_ref().unwrap().payload_offset);
        assert_eq!(10, pkt1.header.borrow().as_ref().unwrap().payload_len);
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
        assert_eq!(seq1, stm.pop().unwrap().seq());
        assert_eq!(seq2, stm.peek_ord().unwrap().seq());
        assert_eq!(seq2, stm.pop().unwrap().seq());
        assert_eq!(seq3, stm.peek_ord().unwrap().seq());
        assert_eq!(seq3, stm.pop().unwrap().seq());
        assert!(stm.is_empty());
        stm.clear();
    }

    // 插入的包有重传 1-10 11-20 1-10 21-30
    #[test]
    fn test_peek_ord_retrans() {
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

        stm.push(pkt1.clone());        
        stm.push(pkt2.clone());
        stm.push(pkt1.clone());
        stm.push(pkt3);

        // dbg!(pkt1.payload_len());        
        // dbg!(seq1);
        // dbg!(seq2);
        // dbg!(seq3);
        
        assert_eq!(seq1, stm.peek_ord().unwrap().seq()); // 看到pkt1
        assert_eq!(seq1, stm.pop().unwrap().seq());      // 弹出pkt1
        assert_eq!(3, stm.len());

        dbg!(stm.peek().unwrap().seq());
        dbg!(stm.len());
        dbg!(stm.expect_seq);
        
        assert_eq!(seq2, stm.peek_ord().unwrap().seq());     // 重复的pkt1被内部释放，看到pkt2
        // assert_eq!(None, stm.pop_ord());                 // 重复的pkt1是top，释放了这个pkt1，返回none
        // peek 没有更新seq
        
        dbg!(stm.peek().unwrap().seq());
        dbg!(stm.len());
        dbg!(stm.expect_seq);
        // assert_eq!(2, stm.len());
        
        // dbg!(stm.len());
        // dbg!(stm.peek().unwrap().seq());
        // assert_eq!(seq2, stm.peek_ord().unwrap().seq())  // pkt2是top
    }

    // 插入的包有覆盖重传 1-10 11-20 15-25 25-35
    #[test]
    fn test_peek_ord_cover() {

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
                 4000,  //desitnation port
                 seq,     //sequence number
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
