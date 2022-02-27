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

use crate::packet::packet_type::PacketType;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::ParseError;

use super::property::Property;
/// This trait provide interface for mapping MQTTv5 packets to human readable structures
/// which can be later modified and used for communication purposes.
pub trait Packet<'a> {
    fn new() -> Self;
    /// Method encode provide way how to transfer Packet struct into Byte array (buffer)
    fn encode(&mut self, buffer: &mut [u8]) -> usize;
    /// Decode method is opposite of encode - decoding Byte array and mapping it into corresponding Packet struct
    fn decode(&mut self, buff_reader: &mut BuffReader<'a>);

    /// Setter method for packet properties len - not all Packet types support this
    fn set_property_len(&mut self, value: u32);
    /// Setter method for packet properties len - not all Packet types support this
    fn get_property_len(&mut self) -> u32;
    /// Method enables pushing new property into packet properties
    fn push_to_properties(&mut self, property: Property<'a>);

    /// Setter for packet fixed header
    fn set_fixed_header(&mut self, header: u8);
    /// Setter for remaining len
    fn set_remaining_len(&mut self, remaining_len: u32);

    /// Method is decoding Byte array pointing to properties into heapless Vec
    /// in packet. If decoding goes wrong method is returning Error
    fn decode_properties(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.set_property_len(buff_reader.read_variable_byte_int().unwrap());
        let mut x: u32 = 0;
        let mut prop: Result<Property, ParseError>;
        if self.get_property_len() != 0 {
            loop {
                let mut res: Property;
                prop = Property::decode(buff_reader);
                if let Ok(res) = prop {
                    log::debug!("Parsed property {:?}", res);
                    x = x + res.len() as u32 + 1;
                    self.push_to_properties(res);
                } else {
                    log::error!("Problem during property decoding");
                }

                if x == self.get_property_len() {
                    break;
                }
            }
        }
    }

    /// Method is decoding packet header into fixed header part and remaining length
    fn decode_fixed_header(&mut self, buff_reader: &mut BuffReader) -> PacketType {
        let first_byte: u8 = buff_reader.read_u8().unwrap();
        self.set_fixed_header(first_byte);
        self.set_remaining_len(buff_reader.read_variable_byte_int().unwrap());
        return PacketType::from(first_byte);
    }
}
