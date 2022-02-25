use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use heapless::Vec;

use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::TopicFilter;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

pub struct UnsubscriptionPacket<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    pub packet_identifier: u16,

    pub property_len: u32,

    // properties
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,

    // topic filter len
    pub topic_filter_len: u16,

    // payload
    pub topic_filters: Vec<TopicFilter<'a>, MAX_FILTERS>,
}

impl<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize>
    UnsubscriptionPacket<'a, MAX_FILTERS, MAX_PROPERTIES>
{

}

impl<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize> Packet<'a>
    for UnsubscriptionPacket<'a, MAX_FILTERS, MAX_PROPERTIES>
{
    fn new() -> Self {
        todo!()
    }

    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] =
            VariableByteIntegerEncoder::encode(self.property_len).unwrap();
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        rm_ln = rm_ln + property_len_len as u32 + 4 + self.topic_filter_len as u32;

        buff_writer.write_u8(self.fixed_header);
        buff_writer.write_variable_byte_int(rm_ln);
        buff_writer.write_u16(self.packet_identifier);
        buff_writer.write_variable_byte_int(self.property_len);
        buff_writer.encode_properties::<MAX_PROPERTIES>(&self.properties);
        buff_writer.write_u16(self.topic_filter_len);
        buff_writer.encode_topic_filters_ref(
            false,
            self.topic_filter_len as usize,
            &self.topic_filters,
        );
        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        log::error!("Unsubscribe packet does not support decode funtion on client!");
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
