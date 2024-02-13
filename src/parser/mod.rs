#![allow(unused)]

use std::pin::Pin;
use std::rc::Rc;
use futures::Future;
use crate::Packet;
use crate::PktStrm;

pub mod smtp;

pub trait Parser { 
    fn stream_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
    
    fn stream_s2c_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
    
    fn stream_bdir_parser(&self, c2s_stream: *const PktStrm, s2c_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }

    fn orderly_pkt_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }

    fn orderly_pkt_s2c_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }

    fn orderly_pkt_bdir_parser(&self, c2s_stream: *const PktStrm, s2c_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
    
    fn raw_order_pkt_c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }

    fn raw_order_pkt_s2c_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }

    fn raw_order_pkt_bdir_parser(&self, c2s_stream: *const PktStrm, s2c_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
}
