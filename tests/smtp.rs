mod common;

use crate::common::*;
use memerge::*;
use memerge::smtp::SmtpParser;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

const SMTP_PORT_NET: u16 = 25;

#[test]
fn test_smtp_parser() {
    let project_root = env::current_dir().unwrap();
    let file_path = project_root.join("tests/smtp.pcap");
    let mut cap = Capture::init(file_path).unwrap();
    let mut task = Task::new_with_parser(SmtpParser);
    let dir = PktDirection::Client2Server;
    
    while task.parser_state(dir.clone()) == TaskState::Start {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        
        let pkt = cap.next_packet(now);
        if pkt.is_none() {
            continue;
        }
        let pkt = pkt.unwrap();
        if pkt.decode().is_err() {
            continue;
        }

        if pkt.header.borrow().as_ref().unwrap().dport() == SMTP_PORT_NET {
            task.run(pkt, dir.clone());
        }
    }
}
