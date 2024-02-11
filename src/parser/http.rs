use std::pin::Pin;
use futures::Future;
use crate::Packet;
use crate::Parser;
use crate::PktStrm;

pub struct HttpParser;

impl Parser for HttpParser {
    fn parser(&self, ptr_stream: *const PktStrm) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {
        })
    }
}
