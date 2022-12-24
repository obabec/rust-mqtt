/*
 * MIT License
 *
 * Copyright (c) [2022] [Ondrej Babec <ond.babec@gmail.com>]
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

use heapless::Vec;

use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;
use crate::utils::types::{BufferError, TopicFilter};

use super::property::Property;

pub struct UnsubscriptionPacket<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize> {
    pub fixed_header: u8,
    pub remain_len: u32,
    pub packet_identifier: u16,
    pub property_len: u32,
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
    pub topic_filter_len: u16,
    pub topic_filters: Vec<TopicFilter<'a>, MAX_FILTERS>,
}

impl<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize>
    UnsubscriptionPacket<'a, MAX_FILTERS, MAX_PROPERTIES>
{
    pub fn add_new_filter(&mut self, topic_name: &'a str) {
        let len = topic_name.len();
        let mut new_filter = TopicFilter::new();
        new_filter.filter.string = topic_name;
        new_filter.filter.len = len as u16;
        new_filter.sub_options |= 0x01;
        self.topic_filters.push(new_filter);
        self.topic_filter_len += 1;
    }
}

impl<'a, const MAX_FILTERS: usize, const MAX_PROPERTIES: usize> Packet<'a>
    for UnsubscriptionPacket<'a, MAX_FILTERS, MAX_PROPERTIES>
{
    fn new() -> Self {
        Self {
            fixed_header: PacketType::Unsubscribe.into(),
            remain_len: 0,
            packet_identifier: 0,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            topic_filter_len: 0,
            topic_filters: Vec::<TopicFilter<'a>, MAX_FILTERS>::new(),
        }
    }

    fn encode(&mut self, buffer: &mut [u8], buffer_len: usize) -> Result<usize, BufferError> {
        let mut buff_writer = BuffWriter::new(buffer, buffer_len);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] = VariableByteIntegerEncoder::encode(self.property_len)?;
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);

        let mut lt = 0;
        let mut filters_len = 0;
        loop {
            filters_len = filters_len + self.topic_filters.get(lt).unwrap().filter.len + 2;
            lt += 1;
            if lt == self.topic_filter_len as usize {
                break;
            }
        }
        rm_ln = rm_ln + property_len_len as u32 + 2 + filters_len as u32;

        buff_writer.write_u8(self.fixed_header)?;
        buff_writer.write_variable_byte_int(rm_ln)?;
        buff_writer.write_u16(self.packet_identifier)?;
        buff_writer.write_variable_byte_int(self.property_len)?;
        buff_writer.write_properties::<MAX_PROPERTIES>(&self.properties)?;
        buff_writer.write_topic_filters_ref(
            false,
            self.topic_filter_len as usize,
            &self.topic_filters,
        )?;
        Ok(buff_writer.position)
    }

    fn decode(&mut self, _buff_reader: &mut BuffReader<'a>) -> Result<(), BufferError> {
        error!("Unsubscribe packet does not support decode funtion on client!");
        Err(BufferError::WrongPacketToDecode)
    }

    fn set_property_len(&mut self, value: u32) {
        self.property_len = value;
    }

    fn get_property_len(&mut self) -> u32 {
        self.property_len
    }

    fn push_to_properties(&mut self, property: Property<'a>) {
        self.properties.push(property);
    }

    fn property_allowed(&mut self, property: &Property<'a>) -> bool {
        property.unsubscribe_property()
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
