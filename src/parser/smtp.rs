use std::pin::Pin;
use futures::Future;
use crate::Packet;
use crate::Parser;
use crate::PktStrm;

pub struct SmtpParser;

impl Parser for SmtpParser {
    fn c2s_parser(&self, stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {
            let mut stream_ref: &mut PktStrm;
            unsafe { stream_ref = &mut *(stream as *mut PktStrm); }
            
        })
    }
}
