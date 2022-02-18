use super::property::Property;
use super::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::ParseError;
use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::TopicFilter;
use heapless::Vec;


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
    fn decode(& mut self, buff_reader: & mut BuffReader<'a>) {
        log::error!("Unsubscribe packet does not support decode funtion on client!");
    }

    fn encode(& mut self, buffer: & mut [u8]) {

    }
}