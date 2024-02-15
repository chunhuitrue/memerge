#![allow(unused)]

use etherparse::*;
use memerge::*;
use std::rc::Rc;

pub fn build_pkt_nodata(seq: u32, fin: bool) -> Rc<Packet> {
    //setup the packet headers
    let mut builder = PacketBuilder::
    ethernet2([1,2,3,4,5,6],     //source mac
              [7,8,9,10,11,12]) //destionation mac
        .ipv4([192,168,1,1], //source ip
              [192,168,1,2], //desitionation ip
              20)            //time to life
        .tcp(25,    //source port 
             htons(4000),  //desitnation port
             htonl(seq),     //sequence number
             1024) //window size
    //set additional tcp header fields
        .ns() //set the ns flag
    //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
        .ack(123) //ack flag + the ack number
        .urg(23) //urg flag + urgent pointer
        .options(&[
            TcpOptionElement::Noop,
            TcpOptionElement::MaximumSegmentSize(1234)
        ]).unwrap();
    if fin {
        builder = builder.fin();            
    }
    
    //payload of the tcp packet
    let payload = [];
    //get some memory to store the result
    let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
    //serialize
    //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
    builder.write(&mut result, &payload).unwrap();
    // println!("result len:{}", result.len());
    
    Packet::new(1, result.len(), &result)
}

// 独立的ack包，没有载荷
pub fn build_pkt_ack(seq: u32, ack_seq: u32) -> Rc<Packet> {
    //setup the packet headers
    let mut builder = PacketBuilder::
    ethernet2([1,2,3,4,5,6],     //source mac
              [7,8,9,10,11,12]) //destionation mac
        .ipv4([192,168,1,1], //source ip
              [192,168,1,2], //desitionation ip
              20)            //time to life
        .tcp(25,    //source port 
             htons(4000),  //desitnation port
             htonl(seq),     //sequence number
             1024) //window size
    //set additional tcp header fields
        .ns() //set the ns flag
    //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
        .ack(htonl(ack_seq)) //ack flag + the ack number
        .urg(23) //urg flag + urgent pointer
        .options(&[
            TcpOptionElement::Noop,
            TcpOptionElement::MaximumSegmentSize(1234)
        ]).unwrap();
    
    //payload of the tcp packet
    let payload = [];
    //get some memory to store the result
    let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
    //serialize
    //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
    builder.write(&mut result, &payload).unwrap();
    // println!("result len:{}", result.len());
    
    Packet::new(1, result.len(), &result)
}

// 独立的syn包，没有载荷
pub fn build_pkt_syn(seq: u32) -> Rc<Packet> {
    //setup the packet headers
    let mut builder = PacketBuilder::
    ethernet2([1,2,3,4,5,6],     //source mac
              [7,8,9,10,11,12]) //destionation mac
        .ipv4([192,168,1,1], //source ip
              [192,168,1,2], //desitionation ip
              20)            //time to life
        .tcp(25,    //source port 
             htons(4000),  //desitnation port
             htonl(seq),     //sequence number
             1024) //window size
    //set additional tcp header fields
        .ns() //set the ns flag
    //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
        .syn()
        .urg(23) //urg flag + urgent pointer
        .options(&[
            TcpOptionElement::Noop,
            TcpOptionElement::MaximumSegmentSize(1234)
        ]).unwrap();
    
    //payload of the tcp packet
    let payload = [];
    //get some memory to store the result
    let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
    //serialize
    //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
    builder.write(&mut result, &payload).unwrap();
    // println!("result len:{}", result.len());
    
    Packet::new(1, result.len(), &result)
}

pub fn make_pkt_data(seq: u32) -> Rc<Packet> {
    build_pkt(seq, false)
}

pub fn build_pkt_line(seq: u32, payload: [u8;10]) -> Rc<Packet> {
    //setup the packet headers
    let mut builder = PacketBuilder::
    ethernet2([1,2,3,4,5,6],     //source mac
              [7,8,9,10,11,12]) //destionation mac
        .ipv4([192,168,1,1], //source ip
              [192,168,1,2], //desitionation ip
              20)            //time to life
        .tcp(25,    //source port 
             htons(4000),  //desitnation port
             htonl(seq),     //sequence number
             1024) //window size
    //set additional tcp header fields
        .ns() //set the ns flag
    //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
        .ack(123) //ack flag + the ack number
        .urg(23) //urg flag + urgent pointer
        .options(&[
            TcpOptionElement::Noop,
            TcpOptionElement::MaximumSegmentSize(1234)
        ]).unwrap();
    
    //get some memory to store the result
    let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
    //serialize
    //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
    builder.write(&mut result, &payload).unwrap();
    
    Packet::new(1, result.len(), &result)
}

// 带载荷，可以带fin
pub fn build_pkt(seq: u32, fin: bool) -> Rc<Packet> {
    //setup the packet headers
    let mut builder = PacketBuilder::
    ethernet2([1,2,3,4,5,6],     //source mac
              [7,8,9,10,11,12]) //destionation mac
        .ipv4([192,168,1,1], //source ip
              [192,168,1,2], //desitionation ip
              20)            //time to life
        .tcp(25,    //source port 
             htons(4000),  //desitnation port
             htonl(seq),     //sequence number
             1024) //window size
    //set additional tcp header fields
        .ns() //set the ns flag
    //supported flags: ns(), fin(), syn(), rst(), psh(), ece(), cwr()
        .ack(123) //ack flag + the ack number
        .urg(23) //urg flag + urgent pointer
        .options(&[
            TcpOptionElement::Noop,
            TcpOptionElement::MaximumSegmentSize(1234)
        ]).unwrap();
    if fin {
        builder = builder.fin();            
    }
    
    //payload of the tcp packet
    let payload = [1,2,3,4,5,6,7,8,9,10];
    //get some memory to store the result
    let mut result = Vec::<u8>::with_capacity(builder.size(payload.len()));
    //serialize
    //this will automatically set all length fields, checksums and identifiers (ethertype & protocol)
    builder.write(&mut result, &payload).unwrap();
    // println!("result len:{}", result.len());
    
    Packet::new(1, result.len(), &result)
}

// 独立的fin包，没有载荷
pub fn build_pkt_fin(seq: u32) -> Rc<Packet> {
    build_pkt_nodata(seq, true)
}

