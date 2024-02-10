#![allow(unused)]

use std::pin::Pin;
use futures::Future;
use crate::Packet;
use crate::PktStrm;
use std::rc::Rc;
use futures::lock::Mutex;
use std::cell::RefCell;

pub mod smtp;
pub mod http;

pub trait Parser { 
    fn parser(&self, stream: Rc<RefCell<PktStrm>>) -> Pin<Box<dyn Future<Output = ()>>>;
}
