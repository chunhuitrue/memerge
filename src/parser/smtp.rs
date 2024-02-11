// use std::pin::Pin;
// use futures::Future;
// use crate::Packet;
// use std::rc::Rc;
// use crate::Parser;
// use crate::PktStrm;
// use futures::lock::Mutex;
// use std::cell::RefCell;

// pub struct SmtpParser;

// impl Parser for SmtpParser {
//     #[allow(clippy::await_holding_refcell_ref)]
//     fn parser(&self, stream: Rc<RefCell<PktStrm>>) -> Pin<Box<dyn Future<Output = ()>>> {
//         Box::pin(async move {
//             // 实现 SMTP 解析逻辑
//         })
//     }
// }
