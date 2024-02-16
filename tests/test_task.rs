mod common;

use futures_util::StreamExt;
use core::{future::Future, pin::Pin};
use memerge::*;
use crate::common::*;

// 简单情况，一个包，带fin
#[test] #[cfg(not(miri))]
fn test_stream() {
    struct StreamTask;
    impl Parser for StreamTask {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                for i in 0..10 {
                    let ret = stream_ref.next().await;
                    println!("i:{}, ret:{}", i, ret.unwrap());
                    assert_eq!(Some(i + 1), ret);
                }
                dbg!("before next None");                
                assert_eq!(None, stream_ref.next().await);
                dbg!("after next None");
            })
        }
    }

    // 1 - 10
    let seq1 = 1;
    let pkt1 = build_pkt(seq1, true);
    let _ = pkt1.decode();

    let dir = PktDirection::Client2Server;        
    let mut task = Task::new(StreamTask);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    task.run(pkt1, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 三个包，最后一个包带fin
#[test] #[cfg(not(miri))]
fn test_stream_3pkt() {
    struct StreamTask3pkt;
    impl Parser for StreamTask3pkt {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
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
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");
    task.run(pkt1, dir.clone());
    println!("run 2");        
    task.run(pkt3, dir.clone());
    println!("run 3");
    task.run(pkt2, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 四个包，最后一个包只带fin
#[test] #[cfg(not(miri))]
fn test_stream_fin() {
    struct StreamTaskFin;
    impl Parser for StreamTaskFin {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
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
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");
    task.run(pkt1, dir.clone());
    println!("run 3");        
    task.run(pkt3, dir.clone());
    println!("run 2");        
    task.run(pkt2, dir.clone());
    println!("run 4");        
    task.run(pkt4, dir.clone());        
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));        
}

// 中间有纯ack包的情况
#[test] #[cfg(not(miri))]    
fn test_stream_ack() {
    struct StreamTaskAck;
    impl Parser for StreamTaskAck {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
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
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    task.run(pkt1, dir.clone());
    task.run(pkt3, dir.clone());
    task.run(pkt2, dir.clone());
    task.run(ack_pkt, dir.clone());
    task.run(pkt4, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// syn包。同时也验证了中间中断，需要多次run的情况
#[test]  #[cfg(not(miri))]
fn test_stream_syn() {
    struct StreamTaskSyn;
    impl Parser for StreamTaskSyn {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
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
                dbg!("up none");
                assert_eq!(None, stream_ref.next().await);
                dbg!("after none");
            })
        }
    }

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

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(StreamTaskSyn);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    task.run(syn_pkt, dir.clone());
    task.run(pkt2, dir.clone());
    task.run(pkt3, dir.clone());
    task.run(pkt1, dir.clone());
    task.run(pkt4, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 4个包，还带syn，fin。看看是否可以跨包readn
#[test] #[cfg(not(miri))]
fn test_readn() {
    struct StreamTaskReadn;
    impl Parser for StreamTaskReadn {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
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
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    task.run(syn_pkt, dir.clone());
    task.run(pkt4, dir.clone());        
    task.run(pkt2, dir.clone());
    task.run(pkt3, dir.clone());
    task.run(pkt1, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}    

// 跨包的行
#[test] #[cfg(not(miri))]
fn test_readline() {
    struct StreamTaskReadLine;
    impl Parser for StreamTaskReadLine {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
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
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    task.run(syn_pkt, dir.clone());
    task.run(pkt4, dir.clone());        
    task.run(pkt1, dir.clone());
    task.run(pkt2, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}    

// 有序包的解码器。3个包有序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt() {
    struct OrdPktTask;
    impl Parser for OrdPktTask {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!("parser. pkt2");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }

                println!("parser. pkt3");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(21, pkt.seq());
                }

                println!("parser. None");
                let ret = stream_ref.next_ord_pkt().await;
                assert_eq!(None, ret);
                dbg!("xxx after none");
            })
        }
    }

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
    let mut task = Task::new(OrdPktTask);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");    
    task.run(pkt1, dir.clone());
    println!("run 2");
    task.run(pkt2, dir.clone());
    println!("run 3");
    task.run(pkt3, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 有序包的解码器。3个包乱序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt_3pkt() {
    struct OrdPktTask3pkt;
    impl Parser for OrdPktTask3pkt {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!(" ");
                println!("parser. pkt2");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }

                println!(" ");
                println!("parser. pkt3");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(21, pkt.seq());
                }
            })
        }
    }
    
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
    let mut task = Task::new(OrdPktTask3pkt);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");    
    task.run(pkt1, dir.clone());
    println!("run 2");
    task.run(pkt3, dir.clone());
    println!("run 3");
    task.run(pkt2, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 有序包的解码器。4个包乱序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt_4pkt() {
    struct OrdPktTask4pkt;
    impl Parser for OrdPktTask4pkt {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!(" ");
                println!("parser. pkt2");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }

                println!(" ");
                println!("parser. pkt3");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(21, pkt.seq());
                }
                
                println!(" ");
                println!("parser. pkt4");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(31, pkt.seq());
                }
            })
        }
    }
    
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
    // 21 - 30
    let seq4 = seq3 + pkt3.payload_len();
    let pkt4 = build_pkt(seq4, true);
    let _ = pkt4.decode();

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(OrdPktTask4pkt);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");    
    task.run(pkt1, dir.clone());
    println!("run 4");
    task.run(pkt4, dir.clone());
    println!("run 2");
    task.run(pkt3, dir.clone());
    println!("run 3");
    task.run(pkt2, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 有序包的解码器。2个包，带syn。正序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt_2pkt_syn() {
    struct OrdPktTask2pktSyn;
    impl Parser for OrdPktTask2pktSyn {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                println!("parser. syn pkt");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!(" ");
                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(2, pkt.seq());
                }
            })
        }
    }
    
    // syn 包seq占一个
    let syn_pkt_seq = 1;
    let syn_pkt = build_pkt_syn(syn_pkt_seq);
    let _ = syn_pkt.decode();
    // 2 - 11
    let seq1 = syn_pkt_seq + 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(OrdPktTask2pktSyn);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");    
    task.run(syn_pkt, dir.clone());
    println!("run 2");
    task.run(pkt1, dir.clone());
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 有序包的解码器。4个包，带syn, seq 0。乱序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt_4pkt_syn() {
    struct OrdPktTask4pktSyn;
    impl Parser for OrdPktTask4pktSyn {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                println!("parser. syn pkt");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("syn pkt. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(0, pkt.seq());
                    assert_eq!(0, pkt.payload_len());                    
                }
                
                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt1. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!("parser. pkt2");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt2. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }

                println!("parser. pkt3");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt3. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(21, pkt.seq());
                }
                
                println!("parser. pkt4");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt4. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(31, pkt.seq());
                }
            })
        }
    }

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
    let pkt4 = build_pkt(seq4, true);
    let _ = pkt4.decode();

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(OrdPktTask4pktSyn);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run syn");
    task.run(syn_pkt, dir.clone());
    println!("run 1");    
    task.run(pkt1, dir.clone());
    println!("run 4");
    task.run(pkt4, dir.clone());
    println!("run 3");
    task.run(pkt3, dir.clone());
    println!("run 2");
    task.run(pkt2, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 有序包的解码器。4个包，带syn, seq 0。带独立fin包。乱序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt_2pkt_fin() {
    struct OrdPktTask2pktSynFin;
    impl Parser for OrdPktTask2pktSynFin {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }
                
                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt1. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!("parser. fin");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("fin. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }
                println!("parser. fin 2");
                
                // let ret = stream_ref.next_ord_pkt().await;
                // assert_eq!(None, ret);
            })
        }
    }

    // 1 - 10
    let seq1 = 1;
    let pkt1 = build_pkt(seq1, false);
    let _ = pkt1.decode();
    // 11
    let fin_seq = seq1 + pkt1.payload_len();
    let fin_pkt = build_pkt_fin(fin_seq);
    let _ = fin_pkt.decode();

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(OrdPktTask2pktSynFin);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run 1");
    task.run(pkt1, dir.clone());
    println!("run fin");
    task.run(fin_pkt, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}

// 有序包的解码器。4个包，带syn, seq 0。带独立fin包。乱序到来
#[test] #[cfg(not(miri))]
fn test_ordpkt_4pkt_fin() {
    struct OrdPktTask4pktSynFin;
    impl Parser for OrdPktTask4pktSynFin {
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                println!("parser. syn pkt");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("syn pkt. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(0, pkt.seq());
                    assert_eq!(0, pkt.payload_len());                    
                }
                
                println!("parser. pkt1");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt1. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
                }

                println!("parser. pkt2");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt2. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }

                println!("parser. pkt3");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt3. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(21, pkt.seq());
                }
                
                println!("parser. pkt4");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("pkt4. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(31, pkt.seq());
                }

                println!("parser. fin");
                if let Some(pkt) = stream_ref.next_ord_pkt().await {
                    println!("fin. seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(41, pkt.seq());
                }
                println!("parser. fin 2");
                
                // let ret = stream_ref.next_ord_pkt().await;
                // assert_eq!(None, ret);
            })
        }
    }

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
    let fin_seq = seq4 + pkt4.payload_len();
    let fin_pkt = build_pkt_fin(fin_seq);
    let _ = fin_pkt.decode();

    let dir = PktDirection::Client2Server;
    let mut task = Task::new(OrdPktTask4pktSynFin);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    println!("run syn");
    task.run(syn_pkt, dir.clone());
    println!("run 1");    
    task.run(pkt1, dir.clone());
    println!("run 4");
    task.run(pkt4, dir.clone());
    println!("run 3");
    task.run(pkt3, dir.clone());
    println!("run 2");
    task.run(pkt2, dir.clone());
    println!("run fin");
    task.run(fin_pkt, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));
}
    
