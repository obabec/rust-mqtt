use heapless::Vec;

use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

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
    pub fn decode_auth_packet(&mut self, buff_reader: &mut BuffReader<'a>) {
        if self.decode_fixed_header(buff_reader) != (PacketType::Pingresp).into() {
            log::error!("Packet you are trying to decode is not PUBACK packet!");
            return;
        }
        self.disconnect_reason = buff_reader.read_u8().unwrap();
        self.decode_properties(buff_reader);
    }

    fn add_reason(& mut self, reason: u8) {
        self.disconnect_reason = reason;
    }
}

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for DisconnectPacket<'a, MAX_PROPERTIES> {
    fn new() -> Self {
        Self {
            fixed_header: PacketType::Disconnect.into(),
            remain_len: 5,
            disconnect_reason: 0x00,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new()
        }
    }

    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);
        buff_writer.write_u8(self.fixed_header);
        let mut property_len_enc = VariableByteIntegerEncoder::encode(self.property_len).unwrap();
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);

        let rm_len: u32 = 1 + self.property_len + property_len_len as u32;
        buff_writer.write_variable_byte_int(rm_len);
        buff_writer.write_u8(self.disconnect_reason);
        buff_writer.write_variable_byte_int(self.property_len);
        buff_writer.encode_properties(&self.properties);
        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.decode_auth_packet(buff_reader);
    }

    fn set_property_len(&mut self, value: u32) {
        self.property_len = value;
    }

    fn get_property_len(&mut self) -> u32 {
        return self.property_len;
    }

    fn push_to_properties(&mut self, property: Property<'a>) {
        self.properties.push(property);
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
