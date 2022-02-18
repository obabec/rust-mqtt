use super::property::Property;
use super::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::ParseError;
use crate::packet::mqtt_packet::Packet;
use heapless::Vec;


pub struct PingrespPacket {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,
}


impl<'a> PingrespPacket {
    pub fn decode_fixed_header(& mut self, buff_reader: & mut BuffReader<'a>) -> PacketType {
        let first_byte: u8 = buff_reader.readU8().unwrap();
        self.fixed_header = first_byte;
        self.remain_len = buff_reader.readVariableByteInt().unwrap();
        return PacketType::from(self.fixed_header);
    }

    pub fn decode_pingresp_packet(& mut self, buff_reader: & mut BuffReader<'a>) {
        if self.decode_fixed_header(buff_reader) != (PacketType::Pingresp).into() {
            log::error!("Packet you are trying to decode is not PUBACK packet!");
            return;
        }
    }
}  

impl<'a> Packet<'a> for PingrespPacket {
    fn decode(& mut self, buff_reader: & mut BuffReader<'a>) {
        self.decode_pingresp_packet(buff_reader);
    }

    fn encode(& mut self, buffer: & mut [u8]) {

    }
}