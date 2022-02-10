use super::property::Property;
use super::packet_type::PacketType;
use heapless::Vec;

pub const MAX_PROPERTIES: usize = 18;

pub struct Packet<'a> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    // variable header
    //optional  prida se pouze u packetu ve kterych ma co delat 
    pub packet_identifier: u16,

    pub protocol_name_len: u16,

    pub protocol_name: u32,

    pub protocol_version: u8,

    pub connect_flags: u8,

    pub keep_alive: u16,
    // property len
    pub property_len: u32,

    // properties
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,

    // Payload of message
    pub payload: &'a mut [u8]
}

impl<'a> Packet<'a> {
    pub fn new(fixed_header: u8, remain_len: u32, packet_identifier: u16, protocol_name_len: u16,
                protocol_name: u32, protocol_version: u8, 
                connect_flags: u8, keep_alive: u16, property_len: u32,
                properties: Vec<Property<'a>, MAX_PROPERTIES>, payload: &'a mut [u8]) -> Self {
        Self { fixed_header, remain_len, packet_identifier, property_len, properties, payload, connect_flags, keep_alive, protocol_name_len, protocol_name, protocol_version }
    }

    pub fn clean(properties: Vec<Property<'a>, MAX_PROPERTIES>, payload: &'a mut [u8]) -> Self {
        Self{ fixed_header: 0x00, remain_len: 0, packet_identifier: 0, property_len: 0, properties, payload, connect_flags: 0, keep_alive: 0, protocol_name_len: 0, protocol_name: 0, protocol_version: 5}
    }

    pub fn encode(&self) {
        log::info!("Encoding!");
    }

    pub fn get_reason_code(&self) {
        log::info!("Getting reason code!");
    }
}