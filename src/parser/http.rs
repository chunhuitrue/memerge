use std::pin::Pin;
use futures::Future;
use futures_channel::mpsc;
use crate::Packet;
use std::rc::Rc;
use crate::Parser;

pub struct HttpParser;

impl Parser for HttpParser {
    fn parse(&self, mut rx: mpsc::Receiver<Rc<Packet>>) -> Pin<Box<dyn Future<Output = ()>>> {
        Box::pin(async move {
        })
    }
}
