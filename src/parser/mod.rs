use futures_channel::mpsc;
use std::pin::Pin;
use futures::Future;
use crate::PktStrm;
use self::smtp::MetaSmtp;

pub mod smtp;

#[derive(Debug)]
pub enum MetaHttp {}

#[derive(Debug)]
pub enum Meta {
    Smtp(MetaSmtp),
    Http(MetaHttp),
}

pub trait Parser { 
    fn c2s_parser(&self, _stream: *const PktStrm, mut _meta_tx: mpsc::Sender<Meta>) -> Pin<Box<dyn Future<Output = ()>>> {        
        Box::pin(async move {})
    }
    
    fn s2c_parser(&self, _stream: *const PktStrm, mut _meta_tx: mpsc::Sender<Meta>) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
    
    fn bdir_parser(&self, _c2s_stream: *const PktStrm, _s2c_stream: *const PktStrm, mut _meta_tx: mpsc::Sender<Meta>) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {})
    }
}
