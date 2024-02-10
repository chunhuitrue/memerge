#![allow(unused)]

use core::{future::Future, pin::Pin, task::{Context, Poll, RawWaker, RawWakerVTable, Waker}};
use std::rc::Rc;
use futures_channel::mpsc;
use crate::Packet;
use crate::Parser;

pub struct Task {
    parser: Pin<Box<dyn Future<Output = ()>>>,
    state: TaskState,
    sender: mpsc::Sender<Rc<Packet>>,
}

impl Task {
    pub fn new(parser: impl Parser) -> Task {
        let (tx, rx) = mpsc::channel(1);
        let parser = parser.parse(rx);        
        Task { parser, state: TaskState::Start, sender: tx }
    }
    
    pub fn run(&mut self, pkt: Rc<Packet>) {
        if self.state == TaskState::End {
            return;
        }

        if let Err(e) = self.sender.try_send(pkt) {
            return;
        }
        
        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.poll(&mut context) {
            Poll::Ready(()) => { self.state = TaskState::End }
            Poll::Pending => {}
        }
    }
    
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.parser.as_mut().poll(context)
    }

    pub fn get_state(&self) -> TaskState {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Start,
    End
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

    struct TestTask;
    impl Parser for TestTask {
        fn parse(&self, mut rx: mpsc::Receiver<Rc<Packet>>) -> Pin<Box<dyn Future<Output = ()>>> {
            Box::pin(async move {
                let Some(_pkt) = rx.next().await else { return; };
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

    #[test]
    fn test_task() {
        let mut task = Task::new(TestTask);
        assert_eq!(TaskState::Start, task.get_state());        
        task.run(build_pkt(1, false));
        assert_eq!(TaskState::End, task.get_state());
        task.run(build_pkt(1, false));
        assert_eq!(TaskState::End, task.get_state());        
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
