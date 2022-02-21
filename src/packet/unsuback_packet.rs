use heapless::Vec;

use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;

use super::packet_type::PacketType;
use super::property::Property;

pub struct UnsubackPacket<'a, const MAX_REASONS: usize, const MAX_PROPERTIES: usize> {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,

    pub packet_identifier: u16,

    pub property_len: u32,

    // properties
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,

    pub reason_codes: Vec<u8, MAX_REASONS>,
}

impl<'a, const MAX_REASONS: usize, const MAX_PROPERTIES: usize>
    UnsubackPacket<'a, MAX_REASONS, MAX_PROPERTIES>
{
    pub fn read_reason_codes(&mut self, buff_reader: &mut BuffReader<'a>) {
        let mut i = 0;
        loop {
            self.reason_codes.push(buff_reader.read_u8().unwrap());
            i = i + 1;
            if i == MAX_REASONS {
                break;
            }
        }
    }

    pub fn decode_suback_packet(&mut self, buff_reader: &mut BuffReader<'a>) {
        if self.decode_fixed_header(buff_reader) != (PacketType::Suback).into() {
            log::error!("Packet you are trying to decode is not UNSUBACK packet!");
            return;
        }
        self.packet_identifier = buff_reader.read_u16().unwrap();
        self.decode_properties(buff_reader);
        self.read_reason_codes(buff_reader);
    }
}

impl<'a, const MAX_REASONS: usize, const MAX_PROPERTIES: usize> Packet<'a>
    for UnsubackPacket<'a, MAX_REASONS, MAX_PROPERTIES>
{
    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        log::error!("UNSUBACK packet does not support encoding!");
        return 0;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.decode_suback_packet(buff_reader);
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
