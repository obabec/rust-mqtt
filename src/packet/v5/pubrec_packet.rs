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
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;
use crate::utils::types::BufferError;

use super::packet_type::PacketType;
use super::property::Property;

pub struct PubrecPacket<'a, const MAX_PROPERTIES: usize> {
    pub fixed_header: u8,
    pub remain_len: u32,
    pub packet_identifier: u16,
    pub reason_code: u8,
    pub property_len: u32,
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
}

impl<'a, const MAX_PROPERTIES: usize> PubrecPacket<'a, MAX_PROPERTIES> {}

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for PubrecPacket<'a, MAX_PROPERTIES> {
    fn new() -> Self {
        Self {
            fixed_header: PacketType::Pubrec.into(),
            remain_len: 0,
            packet_identifier: 0,
            reason_code: 0,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
        }
    }

    fn encode(&mut self, buffer: &mut [u8], buffer_len: usize) -> Result<usize, BufferError> {
        let mut buff_writer = BuffWriter::new(buffer, buffer_len);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] = VariableByteIntegerEncoder::encode(self.property_len)?;
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        rm_ln = rm_ln + property_len_len as u32 + 3;

        buff_writer.write_u8(self.fixed_header)?;
        buff_writer.write_variable_byte_int(rm_ln)?;
        buff_writer.write_u16(self.packet_identifier)?;
        buff_writer.write_u8(self.reason_code)?;
        buff_writer.write_variable_byte_int(self.property_len)?;
        buff_writer.write_properties::<MAX_PROPERTIES>(&self.properties)?;
        Ok(buff_writer.position)
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) -> Result<(), BufferError> {
        if self.decode_fixed_header(buff_reader)? != PacketType::Pubrec {
            error!("Packet you are trying to decode is not PUBREC packet!");
            return Err(BufferError::PacketTypeMismatch);
        }
        self.packet_identifier = buff_reader.read_u16()?;
        self.reason_code = buff_reader.read_u8()?;
        self.decode_properties(buff_reader)
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
        property.pubrec_property()
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
