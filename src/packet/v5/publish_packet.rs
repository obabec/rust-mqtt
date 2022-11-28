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
use crate::packet::v5::publish_packet::QualityOfService::{QoS0, QoS1, QoS2, INVALID};
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;
use crate::utils::types::{BufferError, EncodedString};

use super::packet_type::PacketType;
use super::property::Property;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum QualityOfService {
    QoS0,
    QoS1,
    QoS2,
    INVALID,
}

impl From<u8> for QualityOfService {
    fn from(orig: u8) -> Self {
        match orig {
            0 => QoS0,
            2 => QoS1,
            4 => QoS2,
            _ => INVALID,
        }
    }
}

impl From<QualityOfService> for u8 {
    fn from(value: QualityOfService) -> Self {
        match value {
            QoS0 => 0,
            QoS1 => 2,
            QoS2 => 4,
            INVALID => 3,
        }
    }
}

pub struct PublishPacket<'a, const MAX_PROPERTIES: usize> {
    pub fixed_header: u8,
    pub remain_len: u32,
    pub topic_name: EncodedString<'a>,
    pub packet_identifier: u16,
    pub property_len: u32,
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
    pub message: Option<&'a [u8]>,
}

impl<'a, const MAX_PROPERTIES: usize> PublishPacket<'a, MAX_PROPERTIES> {
    pub fn add_topic_name(&mut self, topic_name: &'a str) {
        self.topic_name.string = topic_name;
        self.topic_name.len = topic_name.len() as u16;
    }

    pub fn add_message(&mut self, message: &'a [u8]) {
        self.message = Some(message);
    }

    pub fn add_qos(&mut self, qos: QualityOfService) {
        self.fixed_header |= <QualityOfService as Into<u8>>::into(qos);
    }

    pub fn add_retain(&mut self, retain: bool) {
        self.fixed_header |= retain as u8
    }

    pub fn add_identifier(&mut self, identifier: u16) {
        self.packet_identifier = identifier;
    }
}

impl<'a, const MAX_PROPERTIES: usize> Packet<'a> for PublishPacket<'a, MAX_PROPERTIES> {
    fn new() -> Self {
        Self {
            fixed_header: PacketType::Publish.into(),
            remain_len: 0,
            topic_name: EncodedString::new(),
            packet_identifier: 1,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            message: None,
        }
    }

    fn encode(&mut self, buffer: &mut [u8], buffer_len: usize) -> Result<usize, BufferError> {
        let mut buff_writer = BuffWriter::new(buffer, buffer_len);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] = VariableByteIntegerEncoder::encode(self.property_len)?;
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        let msg_len = self.message.unwrap().len() as u32;
        rm_ln = rm_ln + property_len_len as u32 + msg_len + self.topic_name.len as u32 + 2;

        buff_writer.write_u8(self.fixed_header)?;
        let qos = self.fixed_header & 0x06;
        if qos != 0 {
            rm_ln += 2;
        }

        buff_writer.write_variable_byte_int(rm_ln)?;
        buff_writer.write_string_ref(&self.topic_name)?;

        if qos != 0 {
            buff_writer.write_u16(self.packet_identifier)?;
        }

        buff_writer.write_variable_byte_int(self.property_len)?;
        buff_writer.write_properties::<MAX_PROPERTIES>(&self.properties)?;
        buff_writer.insert_ref(msg_len as usize, self.message.unwrap())?;
        Ok(buff_writer.position)
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) -> Result<(), BufferError> {
        if self.decode_fixed_header(buff_reader)? != PacketType::Publish {
            error!("Packet you are trying to decode is not PUBLISH packet!");
            return Err(BufferError::PacketTypeMismatch);
        }
        self.topic_name = buff_reader.read_string()?;
        let qos = self.fixed_header & 0x06;
        if qos != 0 {
            // Decode only for QoS 1 / 2
            self.packet_identifier = buff_reader.read_u16()?;
        }
        self.decode_properties(buff_reader)?;
        let mut total_len =
            VariableByteIntegerEncoder::len(VariableByteIntegerEncoder::encode(self.remain_len)?);
        total_len = total_len + 1 + self.remain_len as usize;
        self.message = Some(buff_reader.read_message(total_len));
        Ok(())
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
        property.publish_property()
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
