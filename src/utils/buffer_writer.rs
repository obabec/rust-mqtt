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

use core::str;

use heapless::Vec;

use crate::encoding::variable_byte_integer::{VariableByteInteger, VariableByteIntegerEncoder};
use crate::packet::property::Property;
use crate::utils::buffer_reader::{BinaryData, EncodedString, StringPair, TopicFilter};

pub struct BuffWriter<'a> {
    buffer: &'a mut [u8],
    pub position: usize,
}

impl<'a> BuffWriter<'a> {
    pub fn insert_ref(&mut self, len: usize, array: &[u8]) {
        let mut x: usize = 0;
        if len != 0 {
            loop {
                self.buffer[self.position] = array[x];
                self.increment_position(1);
                x = x + 1;
                if x == len {
                    break;
                }
            }
        }
    }

    pub fn new(buffer: &'a mut [u8]) -> Self {
        return BuffWriter {
            buffer,
            position: 0,
        };
    }

    fn increment_position(&mut self, increment: usize) {
        self.position = self.position + increment;
    }

    pub fn write_u8(&mut self, byte: u8) {
        self.buffer[self.position] = byte;
        self.increment_position(1);
    }

    pub fn write_u16(&mut self, two_bytes: u16) {
        let bytes: [u8; 2] = two_bytes.to_be_bytes();
        self.insert_ref(2, &bytes);
    }

    pub fn write_u32(&mut self, four_bytes: u32) {
        let bytes: [u8; 4] = four_bytes.to_be_bytes();
        self.insert_ref(4, &bytes);
    }

    pub fn write_string_ref(&mut self, str: &EncodedString<'a>) {
        self.write_u16(str.len);
        let bytes = str.string.as_bytes();
        self.insert_ref(str.len as usize, bytes);
    }

    pub fn write_string(&mut self, str: EncodedString<'a>) {
        self.write_u16(str.len);
        let bytes = str.string.as_bytes();
        self.insert_ref(str.len as usize, bytes);
    }

    pub fn write_binary_ref(&mut self, bin: &BinaryData<'a>) {
        self.write_u16(bin.len);
        self.insert_ref(bin.len as usize, bin.bin);
    }

    pub fn write_binary(&mut self, bin: BinaryData<'a>) {
        self.write_u16(bin.len);
        self.insert_ref(bin.len as usize, bin.bin);
    }

    pub fn write_string_pair_ref(&mut self, str_pair: &StringPair<'a>) {
        self.write_string_ref(&str_pair.name);
        self.write_string_ref(&str_pair.value);
    }

    pub fn write_string_pair(&mut self, str_pair: StringPair<'a>) {
        self.write_string(str_pair.name);
        self.write_string(str_pair.value);
    }

    pub fn write_variable_byte_int(&mut self, int: u32) {
        let x: VariableByteInteger = VariableByteIntegerEncoder::encode(int).unwrap();
        let len = VariableByteIntegerEncoder::len(x);
        self.insert_ref(len, &x);
    }

    pub fn encode_property(&mut self, property: &Property<'a>) {
        let x: u8 = property.into();
        self.write_u8(x);
        property.encode(self);
    }

    pub fn encode_properties<const LEN: usize>(&mut self, properties: &Vec<Property<'a>, LEN>) {
        let mut i = 0;
        let len = properties.len();
        if len != 0 {
            loop {
                let prop: &Property = properties.get(i).unwrap();
                self.encode_property(prop);
                i = i + 1;
                if i == len {
                    break;
                }
            }
        }
    }

    fn encode_topic_filter_ref(&mut self, sub: bool, topic_filter: &TopicFilter<'a>) {
        self.write_string_ref(&topic_filter.filter);
        if sub {
            self.write_u8(topic_filter.sub_options)
        }
    }

    pub fn encode_topic_filters_ref<const MAX: usize>(
        &mut self,
        sub: bool,
        len: usize,
        filters: &Vec<TopicFilter<'a>, MAX>,
    ) {
        let mut i = 0;
        loop {
            let topic_filter: &TopicFilter<'a> = filters.get(i).unwrap();
            self.encode_topic_filter_ref(sub, topic_filter);
            i = i + 1;
            if i == len {
                break;
            }
        }
    }
}
