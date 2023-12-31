use packet::Packet;

#[derive(Debug, Clone, Copy)]
pub struct PktStrm {
    // 一个堆排序。或者一个vec
    // a: u16,
}

impl PktStrm {
    pub fn new() -> Self {
        PktStrm {}
    }

    /// 数据包处理，放入缓存
    pub fn put(&mut self, _pkt: &Packet) {
        todo!()
    }

    /// 链接结束
    pub fn finish(&mut self) {
        todo!()
    }

    pub fn timeout(&self) {
    }
}

impl Default for PktStrm {
    fn default() -> Self {
        Self::new()
    }
}
