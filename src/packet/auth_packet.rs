use super::property::Property;
use super::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::ParseError;
use crate::packet::mqtt_packet::Packet;
use heapless::Vec;



pub struct AuthPacket<'a, const MAX_PROPERTIES: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    pub auth_reason: u8,

    pub property_len: u32,

    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
}


impl<'a, const MAX_PROPERTIES: usize> AuthPacket<'a, MAX_PROPERTIES> {

}  

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for AuthPacket<'a, MAX_PROPERTIES> {
    fn decode(& mut self, buff_reader: & mut BuffReader<'a>) {
        log::error!("PingreqPacket packet does not support decode funtion on client!");
    }

    fn encode(& mut self, buffer: & mut [u8]) {

    }
}