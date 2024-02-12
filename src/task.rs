#![allow(unused)]

use core::{future::Future, pin::Pin, task::{Context, Poll, RawWaker, RawWakerVTable, Waker}};
use std::rc::Rc;
use std::fmt;
use crate::Packet;
use crate::PktDirection;
use crate::Parser;
use crate::PktStrm;

pub struct Task {
    c2s_stream: Box<PktStrm>,
    s2c_stream: Box<PktStrm>,    
    c2s_parser: Pin<Box<dyn Future<Output = ()>>>,
    s2c_parser: Pin<Box<dyn Future<Output = ()>>>,
    bdir_parser: Pin<Box<dyn Future<Output = ()>>>,
    c2s_state: TaskState,
    s2c_state: TaskState,
    bdir_state: TaskState,    
}

impl Task {
    pub fn new(parser: impl Parser) -> Task {
        let c2s_stream = Box::new(PktStrm::new());
        let pc2s_stream: *const PktStrm = &*c2s_stream;
        let c2s_parser = parser.c2s_parser(pc2s_stream);

        let s2c_stream = Box::new(PktStrm::new());
        let ps2c_stream: *const PktStrm = &*s2c_stream;
        let s2c_parser = parser.s2c_parser(ps2c_stream);

        let bdir_parser = parser.bdir_parser(pc2s_stream, ps2c_stream);
        
        Task {
            c2s_stream,
            s2c_stream,
            c2s_parser,
            s2c_parser,
            bdir_parser,
            c2s_state: TaskState::Start,
            s2c_state: TaskState::Start,
            bdir_state: TaskState::Start,
        }
    }
    
    pub fn run(&mut self, pkt: Rc<Packet>, pkt_dir: PktDirection) {    
        match pkt_dir {
            PktDirection::Client2Server => {
                self.c2s_stream.push(pkt);                
                self.c2s_run();
            }
            PktDirection::Server2Client => {
                self.s2c_stream.push(pkt);
                self.s2c_run();
            }
            _ => return
        }
        self.bdir_run();
    }
    
    fn c2s_run(&mut self) {
        if self.c2s_state == TaskState::End {
            return;
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.c2s_poll(&mut context) {
            Poll::Ready(()) => { self.c2s_state = TaskState::End }
            Poll::Pending => {}
        }
    }

    fn s2c_run(&mut self) {
        if self.s2c_state == TaskState::End {
            return;
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.s2c_poll(&mut context) {
            Poll::Ready(()) => { self.s2c_state = TaskState::End }
            Poll::Pending => {}
        }
    }

    fn bdir_run(&mut self) {
        if self.bdir_state == TaskState::End {
            return;
        }

        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.bdir_poll(&mut context) {
            Poll::Ready(()) => { self.bdir_state = TaskState::End }
            Poll::Pending => {}
        }
    }
    
    fn c2s_poll(&mut self, context: &mut Context) -> Poll<()> {
        self.c2s_parser.as_mut().poll(context)
    }

    fn s2c_poll(&mut self, context: &mut Context) -> Poll<()> {
        self.s2c_parser.as_mut().poll(context)
    }

    fn bdir_poll(&mut self, context: &mut Context) -> Poll<()> {
        self.bdir_parser.as_mut().poll(context)
    }
    
    pub fn get_state(&self, dir: PktDirection) -> TaskState {
        match dir {
            PktDirection::Client2Server => self.c2s_state,
            PktDirection::Server2Client => self.s2c_state,
            PktDirection::BiDirection => self.bdir_state,
            PktDirection::Unknown => TaskState::Error
        }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("c2s_stream", &self.c2s_stream)
            .field("s2c_stream", &self.s2c_stream)
            .field("c2s_parser", &"c2s_parser")
            .field("s2c_parser", &"s2c_parser")
            .field("bdir_parser", &"bidr_parser")
            .field("c2s_state", &self.c2s_state)
            .field("s2c_state", &self.s2c_state)
            .field("bdir_state", &self.bdir_state)            
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
        fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
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
