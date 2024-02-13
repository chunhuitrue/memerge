#![allow(unused)]

use core::{future::Future, pin::Pin, task::{Context, Poll, RawWaker, RawWakerVTable, Waker}};
use std::rc::Rc;
use std::fmt;
use crate::Packet;
use crate::PktDirection;
use crate::Parser;
use crate::PktStrm;

pub struct Task {
    stream_c2s: Box<PktStrm>,
    stream_s2c: Box<PktStrm>,
    
    stream_c2s_parser: Pin<Box<dyn Future<Output = ()>>>,
    stream_s2c_parser: Pin<Box<dyn Future<Output = ()>>>,
    stream_bdir_parser: Pin<Box<dyn Future<Output = ()>>>,
    stream_c2s_state: TaskState,
    stream_s2c_state: TaskState,
    stream_bdir_state: TaskState,

    // orderly_pkt_c2s_parser: Pin<Box<dyn Future<Output = ()>>>,
    // orderly_pkt_s2c_parser: Pin<Box<dyn Future<Output = ()>>>,
    // orderly_pkt_bdir_parser: Pin<Box<dyn Future<Output = ()>>>,
    // orderly_pkt_c2s_state: TaskState,
    // orderly_pkt_s2c_state: TaskState,
    // orderly_pkt_bdir_state: TaskState,

    // raw_order_pkt_c2s_parser: Pin<Box<dyn Future<Output = ()>>>,
    // raw_order_pkt_s2c_parser: Pin<Box<dyn Future<Output = ()>>>,
    // raw_order_pkt_bdir_parser: Pin<Box<dyn Future<Output = ()>>>,
    // raw_order_pkt_c2s_state: TaskState,
    // raw_order_pkt_s2c_state: TaskState,
    // raw_order_pkt_bdir_state: TaskState,
}

impl Task {
    pub fn new(parser: impl Parser) -> Task {
        let stream_c2s = Box::new(PktStrm::new());
        let stream_s2c = Box::new(PktStrm::new());
        let p_stream_c2s: *const PktStrm = &*stream_c2s;        
        let p_stream_s2c: *const PktStrm = &*stream_s2c;

        let stream_c2s_parser = parser.stream_c2s_parser(p_stream_c2s);
        let stream_s2c_parser = parser.stream_s2c_parser(p_stream_s2c);
        let stream_bdir_parser = parser.stream_bdir_parser(p_stream_c2s, p_stream_s2c);

        // let raw_order_pkt_c2s_parser = parser.raw_order_pkt_c2s_parser(p_stream_c2s);
        // let raw_order_pkt_s2c_parser = parser.raw_order_pkt_s2c_parser(p_stream_s2c);
        // let raw_order_pkt_bdir_parser = parser.raw_order_pkt_bdir_parser(p_stream_c2s, p_stream_s2c);
        
        Task {
            stream_c2s,
            stream_s2c,
            stream_c2s_parser,
            stream_s2c_parser,
            stream_bdir_parser,
            stream_c2s_state: TaskState::Start,
            stream_s2c_state: TaskState::Start,
            stream_bdir_state: TaskState::Start,
        }
    }
    
    pub fn run(&mut self, pkt: Rc<Packet>, pkt_dir: PktDirection) {    
        match pkt_dir {
            PktDirection::Client2Server => {
                self.stream_c2s.push(pkt);                
                self.straem_c2s_run();
            }
            PktDirection::Server2Client => {
                self.stream_s2c.push(pkt);
                self.stream_s2c_run();
            }
            _ => return
        }
        self.stream_bdir_run();
    }
    
    fn straem_c2s_run(&mut self) {
        if self.stream_c2s_state == TaskState::End {
            return;
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.stream_c2s_parser.as_mut().poll(&mut context) {
            Poll::Ready(()) => { self.stream_c2s_state = TaskState::End }
            Poll::Pending => {}
        }
    }

    fn stream_s2c_run(&mut self) {
        if self.stream_s2c_state == TaskState::End {
            return;
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.stream_s2c_parser.as_mut().poll(&mut context) {
            Poll::Ready(()) => { self.stream_s2c_state = TaskState::End }
            Poll::Pending => {}
        }
    }

    fn stream_bdir_run(&mut self) {
        if self.stream_bdir_state == TaskState::End {
            return;
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.stream_bdir_parser.as_mut().poll(&mut context) {
            Poll::Ready(()) => { self.stream_bdir_state = TaskState::End }
            Poll::Pending => {}
        }
    }
    
    pub fn get_state(&self, dir: PktDirection) -> TaskState {
        match dir {
            PktDirection::Client2Server => self.stream_c2s_state,
            PktDirection::Server2Client => self.stream_s2c_state,
            PktDirection::BiDirection => self.stream_bdir_state,
            PktDirection::Unknown => TaskState::Error
        }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("c2s_stream", &self.stream_c2s)
            .field("s2c_stream", &self.stream_s2c)
            .field("c2s_parser", &"c2s_parser")
            .field("s2c_parser", &"s2c_parser")
            .field("bdir_parser", &"bidr_parser")
            .field("c2s_state", &self.stream_c2s_state)
            .field("s2c_state", &self.stream_s2c_state)
            .field("bdir_state", &self.stream_bdir_state)            
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Start,
    End,
    Error
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(std::ptr::null::<()>(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use etherparse::*;
    use futures_util::StreamExt;    
    use crate::{ntohs, ntohl, htons, htonl};
    use crate::PktDirection;
    
    struct TestTask;
    impl Parser for TestTask {
        fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async move {
                let mut stream_ref: &mut PktStrm;
                unsafe { stream_ref = &mut *(stream as *mut PktStrm); }

                let ret = stream_ref.next().await;
                assert_eq!(Some(1), ret);
                let number1 = async_number1().await;
                let number2 = async_number2().await;
                assert_eq!(1, number1);
                assert_eq!(2, number2);
            })
        }
    }
    
    async fn async_number1() -> u32 {
        1
    }

    async fn async_number2() -> u32 {
        2
    }

    #[test] #[cfg(not(miri))]
    fn test_task() {
        let pkt1 = build_pkt(1, false);
        let _ = pkt1.decode();
        let pkt2 = build_pkt(1, false);
        let _ = pkt2.decode();

        let dir = PktDirection::Client2Server;
        let mut task = Task::new(TestTask);
        println!("after task new");
        assert_eq!(TaskState::Start, task.get_state(dir.clone()));
        task.run(pkt1, dir.clone());
        assert_eq!(TaskState::End, task.get_state(dir.clone()));
        task.run(pkt2, dir.clone());
        assert_eq!(TaskState::End, task.get_state(dir));
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
                 htons(4000),  //desitnation port
                 htonl(seq),     //sequence number
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
    
    
}
