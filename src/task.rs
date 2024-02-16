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
    c2s_parser: Option<Pin<Box<dyn Future<Output = ()>>>>,
    s2c_parser: Option<Pin<Box<dyn Future<Output = ()>>>>,
    bdir_parser: Option<Pin<Box<dyn Future<Output = ()>>>>,
    c2s_state: TaskState,
    s2c_state: TaskState,
    bdir_state: TaskState,
}

impl Task {
    pub fn new() -> Task {
        let stream_c2s = Box::new(PktStrm::new());
        let stream_s2c = Box::new(PktStrm::new());
        
        Task {
            stream_c2s,
            stream_s2c,
            c2s_parser: None,
            s2c_parser: None,
            bdir_parser: None,
            c2s_state: TaskState::Start,
            s2c_state: TaskState::Start,
            bdir_state: TaskState::Start,
        }
    }
    
    pub fn new_with_parser(parser: impl Parser) -> Task {
        let stream_c2s = Box::new(PktStrm::new());
        let stream_s2c = Box::new(PktStrm::new());
        let p_stream_c2s: *const PktStrm = &*stream_c2s;        
        let p_stream_s2c: *const PktStrm = &*stream_s2c;
        let c2s_parser = parser.c2s_parser(p_stream_c2s);
        let s2c_parser = parser.s2c_parser(p_stream_s2c);
        let bdir_parser = parser.bdir_parser(p_stream_c2s, p_stream_s2c);
        
        Task {
            stream_c2s,
            stream_s2c,
            c2s_parser: Some(c2s_parser),
            s2c_parser: Some(s2c_parser),
            bdir_parser: Some(bdir_parser),
            c2s_state: TaskState::Start,
            s2c_state: TaskState::Start,
            bdir_state: TaskState::Start,
        }
    }

    pub fn init_parser(&mut self, parser: impl Parser) {
        let p_stream_c2s: *const PktStrm = &*(self.stream_c2s);
        let p_stream_s2c: *const PktStrm = &*(self.stream_s2c);
        let c2s_parser = parser.c2s_parser(p_stream_c2s);
        let s2c_parser = parser.s2c_parser(p_stream_s2c);
        let bdir_parser = parser.bdir_parser(p_stream_c2s, p_stream_s2c);

        self.c2s_parser = Some(c2s_parser);
        self.s2c_parser = Some(s2c_parser);
        self.bdir_parser = Some(bdir_parser);
    }
    
    pub fn run(&mut self, pkt: Rc<Packet>, pkt_dir: PktDirection) {    
        match pkt_dir {
            PktDirection::Client2Server => {
                self.stream_c2s.push(pkt);                
                self.c2s_run();
            }
            PktDirection::Server2Client => {
                self.stream_s2c.push(pkt);
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

        if let Some(parser) = &mut self.c2s_parser {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match Pin::as_mut(parser).poll(&mut context) {            
                Poll::Ready(()) => { self.c2s_state = TaskState::End }
                Poll::Pending => {}
            }  
        }
    }

    fn s2c_run(&mut self) {
        if self.s2c_state == TaskState::End {
            return;
        }

        if let Some(parser) = &mut self.s2c_parser {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match Pin::as_mut(parser).poll(&mut context) {            
                Poll::Ready(()) => { self.s2c_state = TaskState::End }
                Poll::Pending => {}
            }  
        }
    }
    
    fn bdir_run(&mut self) {
        if self.bdir_state == TaskState::End {
            return;
        }

        if let Some(parser) = &mut self.bdir_parser {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match Pin::as_mut(parser).poll(&mut context) {            
                Poll::Ready(()) => { self.bdir_state = TaskState::End }
                Poll::Pending => {}
            }  
        }
    }

    pub fn parser_state(&self, dir: PktDirection) -> TaskState {
        match dir {
            PktDirection::Client2Server => self.c2s_state,
            PktDirection::Server2Client => self.s2c_state,
            PktDirection::BiDirection => self.bdir_state,
            PktDirection::Unknown => TaskState::Error
        }
    }
    
}

impl Default for Task {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("c2s_stream", &self.stream_c2s)
            .field("s2c_stream", &self.stream_s2c)
            
            .field("stream_c2s_state", &self.c2s_state)
            .field("stream_s2c_state", &self.s2c_state)
            .field("stream_bdir_state", &self.bdir_state)
            
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
    use crate::{htons, htonl};
    use crate::PktDirection;

    #[test] #[cfg(not(miri))]
    fn test_task() {
        struct TestTask;
        impl Parser for TestTask {
            fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
                Box::pin(async move {
                    let stream_ref: &mut PktStrm;
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

        let pkt1 = build_pkt(1, false);
        let _ = pkt1.decode();
        let pkt2 = build_pkt(1, false);
        let _ = pkt2.decode();

        let dir = PktDirection::Client2Server;
        let mut task = Task::new_with_parser(TestTask);
        println!("after task new");
        assert_eq!(TaskState::Start, task.parser_state(dir.clone()));
        task.run(pkt1, dir.clone());
        assert_eq!(TaskState::End, task.parser_state(dir.clone()));
        task.run(pkt2, dir.clone());
        assert_eq!(TaskState::End, task.parser_state(dir));
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
