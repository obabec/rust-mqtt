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
use crate::utils::types::{BinaryData, BufferError, EncodedString};

use super::packet_type::PacketType;
use super::property::Property;

pub struct ConnectPacket<'a, const MAX_PROPERTIES: usize, const MAX_WILL_PROPERTIES: usize> {
    pub fixed_header: u8,
    pub remain_len: u32,
    pub protocol_name_len: u16,
    pub protocol_name: u32,
    pub protocol_version: u8,
    pub connect_flags: u8,
    pub keep_alive: u16,
    pub property_len: u32,
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
    pub client_id: EncodedString<'a>,
    pub will_property_len: u32,
    pub will_properties: Vec<Property<'a>, MAX_WILL_PROPERTIES>,
    pub will_topic: EncodedString<'a>,
    pub will_payload: BinaryData<'a>,
    pub username: EncodedString<'a>,
    pub password: BinaryData<'a>,
}

impl<'a, const MAX_PROPERTIES: usize, const MAX_WILL_PROPERTIES: usize>
    ConnectPacket<'a, MAX_PROPERTIES, MAX_WILL_PROPERTIES>
{
    pub fn clean() -> Self {
        let mut x = Self {
            fixed_header: PacketType::Connect.into(),
            remain_len: 0,
            protocol_name_len: 4,
            protocol_name: 0x4d515454,
            protocol_version: 5,
            connect_flags: 0x02,
            keep_alive: 60,
            property_len: 3,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            client_id: EncodedString::new(),
            // Will is not supported as it is un-necessary load for embedded
            will_property_len: 0,
            will_properties: Vec::<Property<'a>, MAX_WILL_PROPERTIES>::new(),
            will_topic: EncodedString::new(),
            will_payload: BinaryData::new(),
            username: EncodedString::new(),

            password: BinaryData::new(),
        };

        let y = Property::ReceiveMaximum(20);
        x.properties.push(y);
        x.client_id.len = 0;
        x
    }

    pub fn add_packet_type(&mut self, new_packet_type: PacketType) {
        self.fixed_header &= 0x0F;
        self.fixed_header |= u8::from(new_packet_type);
    }

    pub fn add_username(&mut self, username: &EncodedString<'a>) {
        self.username = (*username).clone();
        self.connect_flags |= 0x80;
    }

    pub fn add_password(&mut self, password: &BinaryData<'a>) {
        self.password = (*password).clone();
        self.connect_flags |= 0x40;
    }

    pub fn add_will(&mut self, topic: &EncodedString<'a>, payload: &BinaryData<'a>, retain: bool) {
        self.will_topic = topic.clone();
        self.will_payload = payload.clone();
        self.connect_flags |= 0x04;
        if retain {
            self.connect_flags |= 0x20;
        }
    }

    pub fn add_client_id(&mut self, id: &EncodedString<'a>) {
        self.client_id = (*id).clone();
    }
}

impl<'a, const MAX_PROPERTIES: usize, const MAX_WILL_PROPERTIES: usize> Packet<'a>
    for ConnectPacket<'a, MAX_PROPERTIES, MAX_WILL_PROPERTIES>
{
    fn new() -> Self {
        Self {
            fixed_header: PacketType::Connect.into(),
            remain_len: 0,
            protocol_name_len: 4,
            // MQTT
            protocol_name: 0x4d515454,
            protocol_version: 5,
            // Clean start flag
            connect_flags: 0x02,
            keep_alive: 180,
            property_len: 0,
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            client_id: EncodedString::new(),
            will_property_len: 0,
            will_properties: Vec::<Property<'a>, MAX_WILL_PROPERTIES>::new(),
            will_topic: EncodedString::new(),
            will_payload: BinaryData::new(),
            username: EncodedString::new(),
            password: BinaryData::new(),
        }
    }

    fn encode(&mut self, buffer: &mut [u8], buffer_len: usize) -> Result<usize, BufferError> {
        let mut buff_writer = BuffWriter::new(buffer, buffer_len);

        let mut rm_ln = self.property_len;
        let property_len_enc: [u8; 4] = VariableByteIntegerEncoder::encode(self.property_len)?;
        let property_len_len = VariableByteIntegerEncoder::len(property_len_enc);
        // Number 12 => protocol_name_len + protocol_name (6) + protocol_version (1)+ connect_flags (1) + keep_alive (2) + client_id_len (2)
        rm_ln = rm_ln + property_len_len as u32 + 10 + self.client_id.len as u32 + 2;

        if self.connect_flags & 0x04 != 0 {
            let wil_prop_len_enc = VariableByteIntegerEncoder::encode(self.will_property_len)?;
            let wil_prop_len_len = VariableByteIntegerEncoder::len(wil_prop_len_enc);
            rm_ln = rm_ln
                + wil_prop_len_len as u32
                + self.will_property_len
                + self.will_topic.len as u32
                + 2
                + self.will_payload.len as u32
                + 2;
        }
        if (self.connect_flags & 0x80) != 0 {
            rm_ln = rm_ln + self.username.len as u32 + 2;
        }

        if self.connect_flags & 0x40 != 0 {
            rm_ln = rm_ln + self.password.len as u32 + 2;
        }

        buff_writer.write_u8(self.fixed_header)?;
        buff_writer.write_variable_byte_int(rm_ln)?;

        buff_writer.write_u16(self.protocol_name_len)?;
        buff_writer.write_u32(self.protocol_name)?;
        buff_writer.write_u8(self.protocol_version)?;
        buff_writer.write_u8(self.connect_flags)?;
        buff_writer.write_u16(self.keep_alive)?;
        buff_writer.write_variable_byte_int(self.property_len)?;
        buff_writer.write_properties::<MAX_PROPERTIES>(&self.properties)?;
        buff_writer.write_string_ref(&self.client_id)?;

        if self.connect_flags & 0x04 != 0 {
            buff_writer.write_variable_byte_int(self.will_property_len)?;
            buff_writer.write_properties(&self.will_properties)?;
            buff_writer.write_string_ref(&self.will_topic)?;
            buff_writer.write_binary_ref(&self.will_payload)?;
        }

        if self.connect_flags & 0x80 != 0 {
            buff_writer.write_string_ref(&self.username)?;
        }

        if self.connect_flags & 0x40 != 0 {
            buff_writer.write_binary_ref(&self.password)?;
        }

        Ok(buff_writer.position)
    }

    fn decode(&mut self, _buff_reader: &mut BuffReader<'a>) -> Result<(), BufferError> {
        error!("Decode function is not available for control packet!");
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
        property.connect_property()
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
