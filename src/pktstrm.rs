use core::cmp::Ordering;
use std::cmp::Reverse;
use etherparse::TransportHeader;
use std::collections::BinaryHeap;
use std::rc::Rc;
use futures_util::stream::{Stream, StreamExt};
use std::task::Poll;
use futures::future;
use futures::Future;
use futures::future::poll_fn;
use crate::Packet;

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
    
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn timeout(&self) {
    }

    pub async fn readn(&mut self, num: usize) -> Vec<u8> {
        self.take(num).collect::<Vec<u8>>().await
    }
    
    pub async fn readline(&mut self) -> Result<String, std::string::FromUtf8Error> {
        let mut res = self.take_while(|x| future::ready(*x != b'\n')).collect::<Vec<u8>>().await;
        if res.is_empty() {
            String::from_utf8(res)
        } else {
            res.push(b'\n');
            String::from_utf8(res)
        }
    }

    // 异步方式获取下一个原始顺序的包。包含载荷为0的。如果cache中每到来一个包，就调用，那就是原始到来的包顺序
    pub fn next_raw_ord_pkt(&mut self) -> impl Future<Output = Option<Rc<Packet>>> + '_ {
        poll_fn(|_cx| {
            if let Some(pkt) = self.peek_pkt() {
                self.pop_pkt();
                return Poll::Ready(Some(pkt));
            }
            Poll::Pending                
        })
    }    

    // 异步方式获取下一个严格有序的包。包含载荷为0的
    pub fn next_ord_pkt(&mut self) -> impl Future<Output = Option<Rc<Packet>>> + '_ {
        poll_fn(|_cx| {
            if let Some(pkt) = self.pop_ord_pkt() {
                return Poll::Ready(Some(pkt));
            }
            if self.fin {
                return Poll::Ready(None);
            }
            Poll::Pending                
        })
    }    
    
    // 无论是否严格seq连续，peek一个当前最有序的包
    // 不更新next_seq
    pub fn peek_pkt(&self) -> Option<Rc<Packet>> {
        self.cache.peek().map(|rev_pkt| {
            let SeqPacket(pkt) = &rev_pkt.0;
            pkt.clone()
        })
    }
    
    // 无论是否严格seq连续，都pop一个当前包。
    // 注意：next_seq由调用者负责
    pub fn pop_pkt(&mut self) -> Option<Rc<Packet>> {
        if let Some(pkt) = self.cache.pop().map(|rev_pkt| rev_pkt.0.0) {
            if pkt.fin() {
                self.fin = true;
            }
            return Some(pkt);
        }
        None
    }
    
    // top位置去重（并非整个cache内部都去重）
    fn top_pkt_dedup(&mut self) {
        while let Some(pkt) = self.peek_pkt() {
            if (pkt.fin() && pkt.payload_len() == 0) || (pkt.syn() && pkt.payload_len() == 0) {
                return;
            }
            
            if pkt.seq() + pkt.payload_len() <= self.next_seq {
                self.pop_pkt();
                continue;
            }
            return;
        }
    }
    
    // 严格有序。peek一个seq严格有序的包，可能包含payload为0的。如果当前top有序，就peek，否则就none。
    pub fn peek_ord_pkt(&mut self) -> Option<Rc<Packet>> {
        if self.next_seq == 0 {
            if let Some(pkt) = self.peek_pkt() {
                self.next_seq = pkt.seq();
            }
            return self.peek_pkt();
        }

        self.top_pkt_dedup();
        if let Some(pkt) = self.peek_pkt() {
            if pkt.seq() <= self.next_seq {
                return Some(pkt);
            }
        }
        None
    }

    // 严格有序。弹出一个严格有序的包，可能包含载荷为0的。否则为none
    // 并不需要关心fin标记，这不是pkt这一层关心的问题
    pub fn pop_ord_pkt(&mut self) -> Option<Rc<Packet>> {
        if let Some(pkt) = self.peek_ord_pkt() {
            if pkt.syn() && pkt.payload_len() == 0 {
                self.next_seq += 1;                
            } else if self.next_seq == pkt.seq() {
                self.next_seq += pkt.payload_len();                
            } else if self.next_seq > pkt.seq() {
                self.next_seq += pkt.payload_len() - (self.next_seq - pkt.seq());
            }

            return self.pop_pkt();
        }
        None
    }
    
    // 严格有序的数据。peek出一个带数据的严格有序的包。否则为none
    pub fn peek_ord_data(&mut self) -> Option<Rc<Packet>> {
        while let Some(pkt) = self.peek_ord_pkt() {
            if pkt.payload_len() == 0 {
                self.pop_ord_pkt();
                continue;
            }

            break;
        }
        self.peek_ord_pkt()
    }
    
    // 严格有序的数据。pop一个带数据的严格有序的包。否则为none
    pub fn pop_ord_data(&mut self) -> Option<Rc<Packet>> {
        if let Some(pkt) = self.peek_ord_data() {
            match self.next_seq.cmp(&pkt.seq()) {
                std::cmp::Ordering::Equal => self.next_seq += pkt.payload_len(),
                std::cmp::Ordering::Greater => self.next_seq += pkt.payload_len() - (self.next_seq - pkt.seq()),
                std::cmp::Ordering::Less => {},
            }
            return self.pop_pkt();            
        }
        None
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

    fn poll_next(mut self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        if let Some(pkt) = self.peek_ord_data() {
            let index = pkt.header.borrow().as_ref().unwrap().payload_offset as u32 + (self.next_seq - pkt.seq());
            if (index as usize) < pkt.data_len {
                self.next_seq += 1;
                return Poll::Ready(Some(pkt.data[index as usize])); 
            }
        }
        if self.fin {
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}

#[derive(Debug, Clone)]
struct SeqPacket(Rc<Packet>);

impl PartialEq for SeqPacket {
    fn eq(&self, other: &Self) -> bool {
        self.0.seq() == other.0.seq()
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
        self.0.seq().cmp(&other.0.seq())
    }
}

#[cfg(test)]
mod tests {
    use etherparse::*;
    use super::*;    

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
    fn test_peek_pkt() {
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
        
        assert_eq!(1, stm.peek_pkt().unwrap().seq());
        stm.pop_pkt();
        assert_eq!(30, stm.peek_pkt().unwrap().seq());
        stm.pop_pkt();
        assert_eq!(80, stm.peek_pkt().unwrap().seq());
        stm.pop_pkt();        
        assert!(stm.is_empty());
        stm.clear();        
    }

    // 插入的包严格有序 1-10 11-20 21-30
    #[test]
    fn test_peek_ord_pkt() {
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
        
        assert_eq!(seq1, stm.peek_ord_pkt().unwrap().seq());
        assert_eq!(seq1, stm.pop_ord_pkt().unwrap().seq());
        assert_eq!(seq2, stm.peek_ord_pkt().unwrap().seq());
        assert_eq!(seq2, stm.pop_ord_pkt().unwrap().seq());
        assert_eq!(seq3, stm.peek_ord_pkt().unwrap().seq());
        assert_eq!(seq3, stm.pop_ord_pkt().unwrap().seq());
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
        
        assert_eq!(seq1, stm.peek_pkt().unwrap().seq()); // 此时pkt1在top
        assert_eq!(seq1, stm.peek_ord_pkt().unwrap().seq());  // 按有序方式，看到pkt1
        assert_eq!(seq1, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt1, 通过pop_ord_pkt更新next_seq
        assert_eq!(seq2, stm.next_seq); 
        
        assert_eq!(3, stm.len());                         // 此时重复的pkt1，仍在里面，top上
        assert_eq!(seq1, stm.peek_pkt().unwrap().seq());
        assert_eq!(seq2, stm.next_seq);

        dbg!(stm.next_seq);
        assert_eq!(seq2, stm.peek_ord_pkt().unwrap().seq()); // 看到pkt2
        assert_eq!(2, stm.len());                            // peek_ord清理了重复的pkt1
        assert_eq!(seq2, stm.next_seq); //  peek_ord不会更新next_seq

        assert_eq!(seq2, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt2, 通过pop_ord更新next_seq
        assert_eq!(1, stm.len());
        assert_eq!(seq3, stm.next_seq); //  peek_ord不会更新next_seq
        
        assert_eq!(seq3, stm.peek_pkt().unwrap().seq()); // 此时pkt3在top
        assert_eq!(seq3, stm.peek_ord_pkt().unwrap().seq());  // 看到pkt3
        assert_eq!(seq3, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt3, 通过pop_ord更新next_seq
        assert_eq!(seq3 + pkt3.payload_len(), stm.next_seq);

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
        
        assert_eq!(seq1, stm.peek_pkt().unwrap().seq()); // 此时pkt1在top
        assert_eq!(seq1, stm.peek_ord_pkt().unwrap().seq());  // 看到pkt1
        assert_eq!(seq1, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt1, 通过pop_ord更新next_seq
        assert_eq!(pkt1.seq() + pkt1.payload_len(), stm.next_seq);

        assert_eq!(3, stm.len());
        assert_eq!(seq2, stm.peek_pkt().unwrap().seq()); // 此时pkt2在top        
        assert_eq!(seq2, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt2, 通过pop_ord更新next_seq
        assert_eq!(seq2 + pkt2.payload_len(), stm.next_seq);
        
        assert_eq!(2, stm.len());
        assert_eq!(seq3, stm.peek_pkt().unwrap().seq()); // 此时pkt3在top
        assert_eq!(seq3, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt3, 通过pop_ord更新next_seq
        
        assert_eq!(seq3 + pkt3.payload_len(), stm.next_seq);
        assert_eq!(1, stm.len());        
        assert_eq!(seq4, stm.peek_pkt().unwrap().seq()); // 此时pkt4在top
        assert_eq!(seq4, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt4, 通过pop_ord更新next_seq

        assert_eq!(seq4 + pkt4.payload_len(), stm.next_seq);
        assert!(stm.is_empty());
        stm.clear();
    }

    // 有中间丢包
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
        assert_eq!(seq1, stm.peek_pkt().unwrap().seq()); // 此时pkt1在top
        assert_eq!(seq1, stm.peek_ord_pkt().unwrap().seq());  // 看到pkt1
        assert_eq!(seq1, stm.pop_ord_pkt().unwrap().seq());   // 弹出pkt1, 通过pop_ord更新next_seq
        assert_eq!(pkt1.seq() + pkt1.payload_len(), stm.next_seq);

        assert_eq!(1, stm.len());
        assert_eq!(seq3, stm.peek_pkt().unwrap().seq()); // 此时pkt3在top
        assert_eq!(None, stm.peek_ord_pkt());  // 但是通peek_ord_pkt 看不到pkt3
        assert_eq!(None, stm.pop_ord_pkt());
        
        stm.clear();
    }

    // 带数据，带fin。是否可以set fin标记？
    #[test]
    fn test_pkt_fin() {
        // 1 - 10
        let seq1 = 1;
        let pkt1 = build_pkt(seq1, true);
        let _ = pkt1.decode();

        let mut stm = PktStrm::new();
        stm.push(pkt1);

        let ret_pkt1 = stm.pop_ord_data();
        assert_eq!(seq1, ret_pkt1.unwrap().seq());
        assert!(stm.fin);
    }
    
    // 插入的包严格有序 1-10 11-20 21-30, 最后一个有数据而且带fin
    // 用pop_ord_data，才会设置fin
    #[test]    
    fn test_3pkt_fin() {
        let mut stm = PktStrm::new();
        // 1 - 10
        let seq1 = 1;
        let pkt1 = build_pkt(seq1, false);
        let _ = pkt1.decode();
        println!("pkt1. seq1: {}, pkt1 seq: {}, port: {}", seq1, pkt1.seq(), pkt1.header.borrow().as_ref().unwrap().sport());
        // 11 - 20
        let seq2 = seq1 + pkt1.payload_len();
        let pkt2 = build_pkt(seq2, false);
        let _ = pkt2.decode();
        println!("pkt2. seq2: {}, pkt2 seq: {}", seq2, pkt2.seq());
        // 21 - 30
        let seq3 = seq2 + pkt2.payload_len();
        let pkt3 = build_pkt(seq3, true);
        let _ = pkt3.decode();
        println!("pkt3. seq3: {}, pkt3 seq: {}", seq3, pkt3.seq());

        stm.push(pkt2.clone());
        stm.push(pkt3);
        stm.push(pkt1.clone());

        assert_eq!(seq1, stm.pop_ord_data().unwrap().seq());
        assert!(!stm.fin);
        assert_eq!(seq2, stm.pop_ord_data().unwrap().seq());
        assert!(!stm.fin);
        assert_eq!(seq3, stm.pop_ord_data().unwrap().seq());
        assert!(stm.fin);
        assert!(stm.is_empty());        
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
}
