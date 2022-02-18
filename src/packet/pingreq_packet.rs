use super::property::Property;
use super::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::ParseError;
use crate::packet::mqtt_packet::Packet;
use heapless::Vec;


pub const MAX_PROPERTIES: usize = 2;

pub struct PingreqPacket{
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,
}


impl PingreqPacket {

}  

impl<'a> Packet<'a> for PingreqPacket {
    fn decode(& mut self, buff_reader: & mut BuffReader<'a>) {
        log::error!("PingreqPacket packet does not support decode funtion on client!");
    }

    fn encode(& mut self, buffer: & mut [u8]) {

    }
}