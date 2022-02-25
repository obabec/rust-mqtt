use heapless::Vec;

use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

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
    pub fn decode_auth_packet(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.decode_fixed_header(buff_reader);
        self.auth_reason = buff_reader.read_u8().unwrap();
        self.decode_properties(buff_reader);
    }

    pub fn add_reason_code(&mut self, code: u8) {
        if code != 0 && code != 24 && code != 25 {
            log::error!("Provided reason code is not supported!");
            return;
        }
        self.auth_reason = code;
    }

    pub fn add_property(&mut self, p: Property<'a>) {
        if p.auth_property() {
            self.push_to_properties(p);
        } else {
            log::error!("Provided property is not correct AUTH packet property!");
        }
    }
}

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for AuthPacket<'a, MAX_PROPERTIES> {
    fn new() -> Self {
        todo!()
    }
    /*fn new() -> Packet<'a, MAX_PROPERTIES> {
        return AuthPacket { fixed_header: PacketType::Auth.into(), remain_len: 0, auth_reason: 0, property_len: 0, properties: Vec::<Property<'a>, MAX_PROPERTIES>::new() }
    }*/

    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] =
            VariableByteIntegerEncoder::encode(self.property_len).unwrap();
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        rm_ln = rm_ln + property_len_len as u32;
        rm_ln = rm_ln + 1;

        buff_writer.write_u8(self.fixed_header);
        buff_writer.write_variable_byte_int(rm_ln);
        buff_writer.write_u8(self.auth_reason);
        buff_writer.write_variable_byte_int(self.property_len);
        buff_writer.encode_properties::<MAX_PROPERTIES>(&self.properties);
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
