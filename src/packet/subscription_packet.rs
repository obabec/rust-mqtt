use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use heapless::Vec;

use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::TopicFilter;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

pub struct SubscriptionPacket<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize> {
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
    SubscriptionPacket<'a, MAX_FILTERS, MAX_PROPERTIES>
{
    pub fn new() -> Self {
        let mut x = Self {
            fixed_header: PacketType::Subscribe.into(),
            remain_len: 0,
            packet_identifier: 1,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            topic_filter_len: 1,
            topic_filters: Vec::<TopicFilter<'a>, MAX_FILTERS>::new(),
        };
        let mut p = TopicFilter::new();
        p.filter.len = 6;
        p.filter.string = "test/#";
        x.topic_filters.push(p);
        return x;
    }
}

impl<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize> Packet<'a>
    for SubscriptionPacket<'a, MAX_FILTERS, MAX_PROPERTIES>
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

        let mut lt = 0;
        let mut filters_len = 0;
        loop {
            filters_len = filters_len + self.topic_filters.get(lt).unwrap().filter.len + 3;
            lt = lt + 1;
            if lt == self.topic_filter_len as usize {
                break;
            }
        }
        rm_ln = rm_ln + property_len_len as u32 + 2 + filters_len as u32;

        buff_writer.write_u8(self.fixed_header);
        buff_writer.write_variable_byte_int(rm_ln);
        buff_writer.write_u16(self.packet_identifier);
        buff_writer.write_variable_byte_int(self.property_len);
        buff_writer.encode_properties::<MAX_PROPERTIES>(&self.properties);
        buff_writer.encode_topic_filters_ref(
            false,
            self.topic_filter_len as usize,
            &self.topic_filters,
        );
        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        log::error!("Subscribe packet does not support decode funtion on client!");
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
