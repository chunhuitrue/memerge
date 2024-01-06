use etherparse::{PacketHeaders, Ethernet2Header, VlanHeader, IpHeader, TransportHeader};
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

pub const MAX_PACKET_LEN: usize = 2048;

#[derive(Eq, PartialEq, Clone)]
pub struct PktHeader {
    pub link: Option<Ethernet2Header>,
    pub vlan: Option<VlanHeader>,
    pub ip: Option<IpHeader>,
    pub transport: Option<TransportHeader>,
    pub payload_offset: usize
}

#[derive(Eq, PartialEq, Clone)]
pub struct Packet {
    pub timestamp: u128,
    pub caplen: u32,
    pub data: [u8; MAX_PACKET_LEN],
    pub header: RefCell<Option<PktHeader>>
}

impl Packet {
    pub fn new(ts: u128, data_len: u32, data: &[u8]) -> Rc<Packet> {
        let mut pkt = Packet {
            timestamp: ts,
            caplen: data_len,
            data: [0; MAX_PACKET_LEN],
            header: RefCell::new(None)
        };
        let s_data = &mut pkt.data[..data_len.try_into().unwrap()];
        s_data.copy_from_slice(&data[..data_len.try_into().unwrap()]);
        Rc::new(pkt)
    }

    pub fn decode(&self) -> Result<(), PacketError> {
        match PacketHeaders::from_ethernet_slice(self) {
            Ok(headers) => {
                self.header.replace(Some(PktHeader {
                    link: headers.link,
                    vlan: headers.vlan,
                    ip: headers.ip,
                    transport: headers.transport,
                    payload_offset: headers.payload.as_ptr() as usize - self.data.as_ptr() as usize
                }));
                Ok(())
            }
            Err(_) => Err(PacketError::DecodeErr),
        }
    }
}

impl Deref for Packet {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ip: {:?}, Packet: ts: {}, caplen: {}, data: {:?}",
            self.header.borrow().as_ref().unwrap().ip,
            self.timestamp,
            self.caplen,
            self.data
        )
    }
}

pub enum PacketError {
    DecodeErr
}
