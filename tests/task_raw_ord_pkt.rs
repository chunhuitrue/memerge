mod common;

use futures_channel::mpsc;
use core::{future::Future, pin::Pin};
use memerge::*;
use crate::common::*;

// 三个包，最后一个包带fin
#[test] #[cfg(not(miri))]
fn test_raw_ord_3pkt() {
    struct RawOrd3pkt;
    impl Parser for RawOrd3pkt {
        fn c2s_parser(&self, stream: *const PktStrm, _meta_tx: mpsc::Sender<Meta>) -> Pin<Box<dyn Future<Output = ()>>> {        
            Box::pin(async move {
                let stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                if let Some(pkt) = stream_ref.next_raw_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(21, pkt.seq());
                }

                if let Some(pkt) = stream_ref.next_raw_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(11, pkt.seq());
                }

                if let Some(pkt) = stream_ref.next_raw_ord_pkt().await {
                    println!("seq:{}, len:{}", pkt.seq(), pkt.payload_len());
                    assert_eq!(1, pkt.seq());
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
    let mut task = Task::new_with_parser(RawOrd3pkt);
    assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
    task.run(pkt3, dir.clone());
    task.run(pkt2, dir.clone());
    task.run(pkt1, dir.clone());    
    assert_eq!(TaskState::End, task.parser_state(dir.clone()));    
}
