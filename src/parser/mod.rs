#![allow(unused)]

use std::pin::Pin;
use futures::Future;
use crate::Packet;
use crate::PktStrm;

pub mod smtp;
pub mod http;

pub trait Parser { 
    fn parser(&self, ptr_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>>;
}
