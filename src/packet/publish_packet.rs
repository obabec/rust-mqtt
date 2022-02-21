use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use heapless::Vec;

use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

pub struct PublishPacket<'a, const MAX_PROPERTIES: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    pub topic_name: EncodedString<'a>,
    pub packet_identifier: u16,

    pub property_len: u32,

    // properties
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,

    pub message: &'a [u8],
}

impl<'a, const MAX_PROPERTIES: usize> PublishPacket<'a, MAX_PROPERTIES> {
    pub fn new(message: &'a [u8]) -> Self {
        let mut x = Self {
            fixed_header: PacketType::Publish.into(),
            remain_len: 0,
            topic_name: EncodedString::new(),
            packet_identifier: 0,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            message,
        };
        x.topic_name.string = "test/topic";
        x.topic_name.len = 10;
        return x;
    }

    pub fn decode_publish_packet(&mut self, buff_reader: &mut BuffReader<'a>) {
        if self.decode_fixed_header(buff_reader) != (PacketType::Publish).into() {
            log::error!("Packet you are trying to decode is not PUBLISH packet!");
            return;
        }
        self.topic_name = buff_reader.read_string().unwrap();
        let qos = self.fixed_header & 0x03;
        if qos != 0 {
            // je potreba dekodovat jenom pro QoS 1 / 2
            self.packet_identifier = buff_reader.read_u16().unwrap();
        }
        self.decode_properties(buff_reader);
        self.message = buff_reader.read_message();
    }
}

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for PublishPacket<'a, MAX_PROPERTIES> {
    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] =
            VariableByteIntegerEncoder::encode(self.property_len).unwrap();
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        let mut msg_len = self.message.len() as u32;
        rm_ln = rm_ln + property_len_len as u32 + msg_len + self.topic_name.len as u32 + 2;

        buff_writer.write_u8(self.fixed_header);
        let qos = self.fixed_header & 0x03;
        if qos != 0 {
            rm_ln + 2;
        }

        buff_writer.write_variable_byte_int(rm_ln);
        buff_writer.write_string_ref(&self.topic_name);

        if qos != 0 {
            buff_writer.write_u16(self.packet_identifier);
        }

        buff_writer.write_variable_byte_int(self.property_len);
        buff_writer.encode_properties::<MAX_PROPERTIES>(&self.properties);
        buff_writer.insert_ref(msg_len as usize, self.message);
        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.decode_publish_packet(buff_reader);
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
