mod common;

use crate::common::*;
use memerge::*;
use memerge::smtp::{SmtpParser, MetaSmtp};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use std::pin::Pin;
use futures::Future;
use futures_channel::mpsc;

const SMTP_PORT_NET: u16 = 25;

#[test]
fn test_smtp_pkt_parser() {
    struct SmtpPktParser;
    impl Parser for SmtpPktParser {
        fn c2s_parser(&self, stream: *const PktStrm, _meta_tx: mpsc::Sender<Meta>) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async move {
                let stm: &mut PktStrm;
                unsafe { stm = &mut *(stream as *mut PktStrm); }

                let pkt = stm.next_ord_pkt().await.unwrap();
                println!("1. len: {}, seq: {}, raw seq: {}", pkt.payload_len(), pkt.seq(), htonl(pkt.seq()));
                assert_eq!(1341098158, pkt.seq());
                
                let pkt = stm.next_ord_pkt().await.unwrap();
                println!("2. len: {}, seq: {}, raw seq: {}", pkt.payload_len(), pkt.seq(), htonl(pkt.seq()));
                assert_eq!(1341098176, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341098188, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();                
                assert_eq!(1341098222, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();                
                assert_eq!(1341098236, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341098286, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341098323, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341098329, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341098728, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341100152, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341101576, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341102823, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341104247, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341105671, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();                
                assert_eq!(1341106918, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341108342, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341108886, pkt.seq());

                let pkt = stm.next_ord_pkt().await.unwrap();
                assert_eq!(1341108891, pkt.seq());
            })
        }
    }
    
    let project_root = env::current_dir().unwrap();
    let file_path = project_root.join("tests/smtp.pcap");
    let mut cap = Capture::init(file_path).unwrap();
    let mut task = Task::new_with_parser(SmtpPktParser);
    let dir = PktDirection::Client2Server;

    loop {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let pkt = cap.next_packet(now);
        if pkt.is_none() {
            println!("no pkt quit");
            break;
        }
        let pkt = pkt.unwrap();
        if pkt.decode().is_err() {
            println!("decode continue");            
            continue;
        }

        if pkt.header.borrow().as_ref().unwrap().dport() == SMTP_PORT_NET {
            task.run(pkt, dir.clone());
            println!("run d. stm len: {}", task.steeam_len(dir.clone()));
        }
    }
    println!("read pkt end");
}

#[test]
fn test_smtp_parser() {
    let project_root = env::current_dir().unwrap();
    let file_path = project_root.join("tests/smtp.pcap");
    let mut cap = Capture::init(file_path).unwrap();
    let mut task = Task::new_with_parser(SmtpParser);
    let dir = PktDirection::Client2Server;

    loop {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let pkt = cap.next_packet(now);
        if pkt.is_none() {
            break;
        }
        let pkt = pkt.unwrap();
        if pkt.decode().is_err() {
            continue;
        }

        if pkt.header.borrow().as_ref().unwrap().dport() == SMTP_PORT_NET {
            task.run(pkt, dir.clone());
            meta_recver(&mut task);
        }
    }
}

fn meta_recver(task: &mut Task) {
    while let Some(meta) = task.get_meta() {
        match meta {
            memerge::Meta::Smtp(smtp) => {
                meta_smtp_recver(smtp)
            }
            memerge::Meta::Http(_) => {}
        }
    }
}

fn meta_smtp_recver(smtp: MetaSmtp) {
    println!("recv meta_smtp: {:?}", smtp);
    match smtp {
        MetaSmtp::User(user) => assert_eq!("dXNlcjEyMzQ1QGV4YW1wbGUxMjMuY29t", user),
        MetaSmtp::Pass(pass) => assert_eq!("MTIzNDU2Nzg=", pass),
        MetaSmtp::MailFrom(mail, mail_size) => {
            assert_eq!("user12345@example123.com", mail);
            assert_eq!(10557, mail_size);
        }
        MetaSmtp::RcptTo(mail) => {
            assert_eq!("user12345@example123.com", mail);
        }
        MetaSmtp::Subject(subject) => {
            assert_eq!("biaoti", subject);
        }
    }
}
