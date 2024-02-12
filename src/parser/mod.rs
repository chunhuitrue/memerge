#![allow(unused)]

use std::pin::Pin;
use futures::Future;
use crate::Packet;
use crate::PktStrm;

pub mod smtp;

pub trait Parser { 
    fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
    
    fn s2c_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
    
    fn bdir_parser(&self, c2s_stream: *const PktStrm, s2c_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
}
