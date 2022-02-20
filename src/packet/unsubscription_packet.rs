use heapless::Vec;

use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::TopicFilter;

use super::packet_type::PacketType;
use super::property::Property;

pub const MAX_PROPERTIES: usize = 20;

pub struct UnsubscriptionPacket<'a, const MAX_FILTERS: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    pub packet_identifier: u16,

    pub property_len: u32,

    // properties
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,

    // topic filter len
    pub topic_filter_let: u16,

    // payload
    pub topic_filters: Vec<TopicFilter<'a>, MAX_FILTERS>,
}

impl<'a, const MAX_FILTERS: usize> UnsubscriptionPacket<'a, MAX_FILTERS> {
    /*pub fn new() -> Self {

    }*/
}

impl<'a, const MAX_FILTERS: usize> Packet<'a> for UnsubscriptionPacket<'a, MAX_FILTERS> {
    fn encode(&mut self, buffer: &mut [u8]) {}

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

    fn set_fixed_header(& mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(& mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
