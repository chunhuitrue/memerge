use std::pin::Pin;
use futures::Future;
use crate::Packet;
use crate::Parser;
use crate::PktStrm;

pub struct SmtpParser;

impl Parser for SmtpParser {
    fn parser(&self, ptr_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {    
        Box::pin(async move {
        })
    }
}
