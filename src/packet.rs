#![allow(unused)]

use etherparse::{PacketHeaders, Ethernet2Header, VlanHeader, IpHeader, TransportHeader};
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;
use crate::util::*;

pub const MAX_PACKET_LEN: usize = 2048;

#[derive(Eq, PartialEq, Clone)]
pub struct PktHeader {
    pub link: Option<Ethernet2Header>,
    pub vlan: Option<VlanHeader>,
    pub ip: Option<IpHeader>,
    pub transport: Option<TransportHeader>,
    pub payload_offset: usize,
    pub payload_len: usize
}

impl PktHeader {
    // 返回tcp或者udp的sport
    pub fn sport(&self) -> u16 {
        match &self.transport {
            Some(TransportHeader::Udp(udph)) => udph.source_port,
            Some(TransportHeader::Tcp(tcph)) => tcph.source_port,
            _ => 0
        }
    }

    // 返回tcp或者udp的sport
    pub fn dport(&self) -> u16 {
        match &self.transport {
            Some(TransportHeader::Udp(udph)) => udph.destination_port,
            Some(TransportHeader::Tcp(tcph)) => tcph.destination_port,
            _ => 0
        }
    }
    
}

#[derive(Eq, PartialEq, Clone)]
pub struct Packet {
    pub timestamp: u128,
    pub data: [u8; MAX_PACKET_LEN],
    pub data_len: usize,
    pub header: RefCell<Option<PktHeader>>
}

impl Packet {
    pub fn new(ts: u128, len: usize, data: &[u8]) -> Rc<Packet> {
        let mut pkt = Packet {
            timestamp: ts,
            data_len: len,
            data: [0; MAX_PACKET_LEN],
            header: RefCell::new(None)
        };
        let s_data = &mut pkt.data[..len];
        s_data.copy_from_slice(&data[..len]);
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
                    payload_offset: headers.payload.as_ptr() as usize - self.data.as_ptr() as usize,
                    payload_len: self.data_len - (headers.payload.as_ptr() as usize - self.data.as_ptr() as usize)
                }));
                Ok(())
            }
            Err(_) => Err(PacketError::DecodeErr),
        }
    }

    pub fn seq(&self) -> u32 {
        if let Some(TransportHeader::Tcp(tcph)) = &self.header.borrow().as_ref().unwrap().transport {
            ntohl(tcph.sequence_number)
        } else {
            0
        }
    }

    pub fn payload_len(&self) -> u32 {
        self.header.borrow().as_ref().unwrap().payload_len.try_into().unwrap()
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
            self.data_len,
            self.data
        )
    }
}

pub enum PacketError {
    DecodeErr
}
