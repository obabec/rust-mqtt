use super::property::Property;
use super::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::ParseError;
use crate::packet::mqtt_packet::Packet;
use heapless::Vec;



pub struct DisconnectPacket<'a, const MAX_PROPERTIES: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    pub disconnect_reason: u8,

    pub property_len: u32,

    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
}


impl<'a, const MAX_PROPERTIES: usize> DisconnectPacket<'a, MAX_PROPERTIES> {
    pub fn decode_properties(& mut self, buff_reader: & mut BuffReader<'a>) {
        self.property_len = buff_reader.readVariableByteInt().unwrap();
        let mut x: u32 = 0;
        let mut prop: Result<Property, ParseError>;
        loop {
            let mut res: Property;
            prop = Property::decode(buff_reader);
            if let Ok(res) = prop {
                log::info!("Parsed property {:?}", res);
                x = x + res.len() as u32 + 1;
                self.properties.push(res);
            } else {
                // error handlo
                log::error!("Problem during property decoding");
            }
            
            if x == self.property_len {
                break;
            }
        }
    }

    pub fn decode_fixed_header(& mut self, buff_reader: & mut BuffReader<'a>) -> PacketType {
        let first_byte: u8 = buff_reader.readU8().unwrap();
        self.fixed_header = first_byte;
        self.remain_len = buff_reader.readVariableByteInt().unwrap();
        return PacketType::from(self.fixed_header);
    }

    pub fn decode_auth_packet(& mut self, buff_reader: & mut BuffReader<'a>) {
        if self.decode_fixed_header(buff_reader) != (PacketType::Pingresp).into() {
            log::error!("Packet you are trying to decode is not PUBACK packet!");
            return;
        }
        self.disconnect_reason = buff_reader.readU8().unwrap();
        self.decode_properties(buff_reader);
    }
}  

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for DisconnectPacket<'a, MAX_PROPERTIES> {
    fn decode(& mut self, buff_reader: & mut BuffReader<'a>) {
        self.decode_auth_packet(buff_reader);
    }

    fn encode(& mut self, buffer: & mut [u8]) {

    }
}