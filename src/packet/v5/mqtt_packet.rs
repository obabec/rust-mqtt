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

use crate::packet::v5::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::types::BufferError;

use super::property::Property;

/// This trait provide interface for mapping MQTTv5 packets to human readable structures
/// which can be later modified and used for communication purposes.
pub trait Packet<'a> {
    fn new() -> Self;
    /// Method encode provide way how to transfer Packet struct into Byte array (buffer)
    fn encode(&mut self, buffer: &mut [u8], buff_len: usize) -> Result<usize, BufferError>;
    /// Decode method is opposite of encode - decoding Byte array and mapping it into corresponding Packet struct
    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) -> Result<(), BufferError>;

    /// Setter method for packet properties len - not all Packet types support this
    fn set_property_len(&mut self, value: u32);
    /// Setter method for packet properties len - not all Packet types support this
    fn get_property_len(&mut self) -> u32;
    /// Method enables pushing new property into packet properties
    fn push_to_properties(&mut self, property: Property<'a>);
    /// Returns if property is allowed for packet
    fn property_allowed(&mut self, property: &Property<'a>) -> bool;
    /// Method enables adding properties from client config - each packet decides if property can be used with that or not
    fn add_properties<const MAX_PROPERTIES: usize>(
        &mut self,
        properties: &Vec<Property<'a>, MAX_PROPERTIES>,
    ) -> u32 {
        let mut res: u32 = 0;
        for prop in properties.iter() {
            if self.property_allowed(prop) {
                self.push_to_properties((*prop).clone());
                res = res + prop.encoded_len() as u32 + 1;
            }
        }
        res
    }

    /// Setter for packet fixed header
    fn set_fixed_header(&mut self, header: u8);
    /// Setter for remaining len
    fn set_remaining_len(&mut self, remaining_len: u32);

    /// Method is decoding Byte array pointing to properties into heapless Vec
    /// in packet. If decoding goes wrong method is returning Error
    fn decode_properties(&mut self, buff_reader: &mut BuffReader<'a>) -> Result<(), BufferError> {
        self.set_property_len(buff_reader.read_variable_byte_int()?);
        let mut x: u32 = 0;
        let mut prop: Property;
        if self.get_property_len() != 0 {
            loop {
                prop = Property::decode(buff_reader)?;
                //debug!("Parsed property {:?}", prop);
                x = x + prop.encoded_len() as u32 + 1;
                self.push_to_properties(prop);

                if x == self.get_property_len() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Method is decoding packet header into fixed header part and remaining length
    fn decode_fixed_header(
        &mut self,
        buff_reader: &mut BuffReader,
    ) -> Result<PacketType, BufferError> {
        let first_byte: u8 = buff_reader.read_u8()?;
        trace!("First byte of accepted packet: {:02X}", first_byte);
        self.set_fixed_header(first_byte);
        self.set_remaining_len(buff_reader.read_variable_byte_int()?);
        Ok(PacketType::from(first_byte))
    }
}
