use futures_util::StreamExt;
use core::{future::Future, pin::Pin};
use memerge::*;
use crate::common::*;

mod common;

struct StreamTask;
impl Parser for StreamTask {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

            for i in 0..10 {
                let ret = stream_ref.next().await;
                println!("i:{}, ret:{}", i, ret.unwrap());
                assert_eq!(Some(i + 1), ret);
            }
            assert_eq!(None, stream_ref.next().await);
        })
    }
}

// 简单情况，一个包，带fin
#[test] #[cfg(not(miri))]
fn test_stream() {
    // 1 - 10
    let seq1 = 1;
    let pkt1 = build_pkt(seq1, true);
    let _ = pkt1.decode();

    let dir = PktDirection::Client2Server;        
    let mut task = Task::new(StreamTask);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    task.run(pkt1, dir.clone());
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));
}

struct StreamTask3pkt;
impl Parser for StreamTask3pkt {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }
            
            for j in 0..3 {
                println!("j:{}", j);
                for i in 0..10 {
                    println!("  up next");
                    let ret = stream_ref.next().await;
                    println!("  i:{}, ret:{}", i, ret.unwrap());
                    assert_eq!(Some(i + 1), ret);
                }
            }
            assert_eq!(None, stream_ref.next().await);                
        })
    }
}

// 三个包，最后一个包带fin
#[test] #[cfg(not(miri))]
fn test_stream_3pkt() {
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

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTask3pkt);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    println!("run 1");
    task.run(pkt1, dir.clone());
    println!("run 2");        
    task.run(pkt3, dir.clone());
    println!("run 3");
    task.run(pkt2, dir.clone());
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));
}

struct StreamTaskFin;
impl Parser for StreamTaskFin {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }
            
            for _j in 0..3 {
                for i in 0..10 {
                    let ret = stream_ref.next().await;
                    println!("i:{}, ret:{}", i, ret.unwrap());
                    assert_eq!(Some(i + 1), ret);
                }
            }
            assert_eq!(None, stream_ref.next().await);
        })
    }
}

// 四个包，最后一个包只带fin
#[test] #[cfg(not(miri))]
fn test_stream_fin() {
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

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTaskFin);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    println!("run 1");
    task.run(pkt1, dir.clone());
    println!("run 3");        
    task.run(pkt3, dir.clone());
    println!("run 2");        
    task.run(pkt2, dir.clone());
    println!("run 4");        
    task.run(pkt4, dir.clone());        
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));        
}

struct StreamTaskAck;
impl Parser for StreamTaskAck {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

            for _j in 0..3 {
                for i in 0..10 {
                    let ret = stream_ref.next().await;
                    println!("i:{}, ret:{}", i, ret.unwrap());
                    assert_eq!(Some(i + 1), ret);
                }
            }
            assert_eq!(None, stream_ref.next().await);
        })
    }
}

// 中间有纯ack包的情况
#[test] #[cfg(not(miri))]    
fn test_stream_ack() {
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

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTaskAck);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    task.run(pkt1, dir.clone());
    task.run(pkt3, dir.clone());
    task.run(pkt2, dir.clone());
    task.run(ack_pkt, dir.clone());
    task.run(pkt4, dir.clone());
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));
}

struct StreamTaskSyn;
impl Parser for StreamTaskSyn {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

            for _j in 0..3 {
                for i in 0..10 {
                    let ret = stream_ref.next().await;
                    println!("i:{}, ret:{}", i, ret.unwrap());
                    assert_eq!(Some(i + 1), ret);
                }
            }
            assert_eq!(None, stream_ref.next().await);
        })
    }
}

// syn包。同时也验证了中间中断，需要多次run的情况
#[test]  #[cfg(not(miri))]
fn test_stream_syn() {
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

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTaskSyn);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    task.run(syn_pkt, dir.clone());
    task.run(pkt2, dir.clone());
    task.run(pkt3, dir.clone());
    task.run(pkt1, dir.clone());
    task.run(pkt4, dir.clone());
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));
}

struct StreamTaskReadn;
impl Parser for StreamTaskReadn {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

            let res = stream_ref.readn(5).await;
            assert_eq!(vec![1,2,3,4,5], res);
            let res = stream_ref.readn(10).await;
            assert_eq!(vec![6,7,8,9,10,1,2,3,4,5], res);
            let res = stream_ref.readn(15).await;
            assert_eq!(vec![6,7,8,9,10,1,2,3,4,5,6,7,8,9,10], res);
            let res = stream_ref.readn(10).await;
            assert_eq!(Vec::<u8>::new(), res);
        })
    }
}
    
// 4个包，还带syn，fin。看看是否可以跨包readn
#[test] #[cfg(not(miri))]
fn test_readn() {
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

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTaskReadn);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    task.run(syn_pkt, dir.clone());
    task.run(pkt4, dir.clone());        
    task.run(pkt2, dir.clone());
    task.run(pkt3, dir.clone());
    task.run(pkt1, dir.clone());
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));
}    

struct StreamTaskReadLine;
impl Parser for StreamTaskReadLine {
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {
            let stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

            let res = stream_ref.readline().await;
            assert_eq!(*b"1234\r\n", res.as_slice());
            let res = stream_ref.readline().await;
            assert_eq!(*b"56781234\r\n", res.as_slice());
            let res = stream_ref.readline().await;
            assert_eq!(*b"56\r\n", res.as_slice());
            let res = stream_ref.readline().await;        
            assert_eq!(Vec::<u8>::new(), res);
        })
    }
}

// 跨包的行
#[test] #[cfg(not(miri))]
fn test_readline() {
    // syn 包seq占一个
    let syn_pkt_seq = 1;
    let syn_pkt = build_pkt_syn(syn_pkt_seq);
    let _ = syn_pkt.decode();
    // 1 - 10
    let seq1 = syn_pkt_seq + 1;
    let pkt1 = build_pkt_line(seq1, *b"1234\r\n5678");
    let _ = pkt1.decode();
    // 11 - 20
    let seq2 = seq1 + pkt1.payload_len();
    let pkt2 = build_pkt_line(seq2, *b"1234\r\n56\r\n");
    let _ = pkt2.decode();
    // 21 无数据，fin
    let seq4 = seq2 + pkt2.payload_len();
    let pkt4 = build_pkt_nodata(seq4, true);
    let _ = pkt4.decode();

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTaskReadLine);
    assert_eq!(TaskState::Start, task.stream_parser_state(dir.clone()));
    task.run(syn_pkt, dir.clone());
    task.run(pkt4, dir.clone());        
    task.run(pkt1, dir.clone());
    task.run(pkt2, dir.clone());
    assert_eq!(TaskState::End, task.stream_parser_state(dir.clone()));
}    
