#![allow(unused)]

use std::pin::Pin;
use futures::Future;
use futures_channel::mpsc;
use crate::Packet;
use std::rc::Rc;

pub mod smtp;
pub mod http;

pub trait Parser { 
    fn parse(&self, rx: mpsc::Receiver<Rc<Packet>>) -> Pin<Box<dyn Future<Output = ()>>>;    
}
