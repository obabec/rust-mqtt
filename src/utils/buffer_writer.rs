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

use crate::encoding::variable_byte_integer::{VariableByteInteger, VariableByteIntegerEncoder};
use crate::packet::v5::property::Property;
use crate::utils::types::{BinaryData, BufferError, EncodedString, StringPair, TopicFilter};

#[derive(Debug, Clone, Copy)]
pub struct RemLenError;

/// Buff writer is writing corresponding types to buffer (Byte array) and stores current position
/// (later as cursor)
pub struct BuffWriter<'a> {
    buffer: &'a mut [u8],
    pub position: usize,
    len: usize,
}

impl<'a> BuffWriter<'a> {
    pub fn new(buffer: &'a mut [u8], buff_len: usize) -> Self {
        Self {
            buffer,
            position: 0,
            len: buff_len,
        }
    }

    fn increment_position(&mut self, increment: usize) {
        self.position += increment;
    }

    /// Returns n-th Byte from the buffer to which is currently written.
    pub fn get_n_byte(&mut self, n: usize) -> u8 {
        if self.position >= n {
            self.buffer[n]
        } else {
            0
        }
    }

    /// Return the remaining lenght of the packet from the buffer to which is packet written.
    pub fn get_rem_len(&mut self) -> Result<VariableByteInteger, RemLenError> {
        let max = if self.position >= 5 {
            4
        } else {
            self.position - 1
        };
        let mut i = 1;
        let mut len: VariableByteInteger = [0; 4];
        loop {
            len[i - 1] = self.buffer[i];
            if len[i - 1] & 0x80 == 0 {
                return Ok(len);
            }
            if len[i - 1] & 0x80 != 0 && i == max && i != 4 {
                return Err(RemLenError);
            }
            if i == max {
                return Ok(len);
            }
            i += 1;
        }
    }

    /// Writes (part of) an array to the buffer.
    pub fn insert_ref(&mut self, len: usize, array: &[u8]) -> Result<(), BufferError> {
        if self.position + len > self.len {
            return Err(BufferError::InsufficientBufferSize);
        }
        self.buffer[self.position..self.position+len].copy_from_slice(&array[0..len]);
        self.increment_position(len);
        Ok(())
    }

    /// Writes a single Byte to the buffer.
    pub fn write_u8(&mut self, byte: u8) -> Result<(), BufferError> {
        if self.position >= self.len {
            Err(BufferError::InsufficientBufferSize)
        } else {
            self.buffer[self.position] = byte;
            self.increment_position(1);
            Ok(())
        }
    }

    /// Writes the two Byte value to the buffer.
    pub fn write_u16(&mut self, two_bytes: u16) -> Result<(), BufferError> {
        let bytes: [u8; 2] = two_bytes.to_be_bytes();
        self.insert_ref(2, &bytes)
    }

    /// Writes the four Byte value to the buffer.
    pub fn write_u32(&mut self, four_bytes: u32) -> Result<(), BufferError> {
        let bytes: [u8; 4] = four_bytes.to_be_bytes();
        self.insert_ref(4, &bytes)
    }

    /// Writes the UTF-8 string type to the buffer.
    pub fn write_string_ref(&mut self, str: &EncodedString<'a>) -> Result<(), BufferError> {
        self.write_u16(str.len)?;
        if str.len != 0 {
            let bytes = str.string.as_bytes();
            return self.insert_ref(str.len as usize, bytes);
        }
        Ok(())
    }

    /// Writes BinaryData to the buffer.
    pub fn write_binary_ref(&mut self, bin: &BinaryData<'a>) -> Result<(), BufferError> {
        self.write_u16(bin.len)?;
        self.insert_ref(bin.len as usize, bin.bin)
    }

    /// Writes the string pair to the buffer.
    pub fn write_string_pair_ref(&mut self, str_pair: &StringPair<'a>) -> Result<(), BufferError> {
        self.write_string_ref(&str_pair.name)?;
        self.write_string_ref(&str_pair.value)
    }

    /// Encodes the u32 value into the VariableByteInteger and this value writes to the buffer.
    pub fn write_variable_byte_int(&mut self, int: u32) -> Result<(), BufferError> {
        let x: VariableByteInteger = VariableByteIntegerEncoder::encode(int)?;
        let len = VariableByteIntegerEncoder::len(x);
        self.insert_ref(len, &x)
    }

    fn write_property(&mut self, property: &Property<'a>) -> Result<(), BufferError> {
        let x: u8 = property.into();
        self.write_u8(x)?;
        property.encode(self)
    }

    /// Writes all properties from the `properties` Vec into the buffer.
    pub fn write_properties<const LEN: usize>(
        &mut self,
        properties: &Vec<Property<'a>, LEN>,
    ) -> Result<(), BufferError> {
        for prop in properties.iter() {
            self.write_property(prop)?;
        }
        Ok(())
    }

    /// Writes the MQTT `TopicFilter` into the buffer. If the `sub` option is set to `false`, it will
    /// not write the `sub_options` only topic name.
    fn write_topic_filter_ref(
        &mut self,
        sub: bool,
        topic_filter: &TopicFilter<'a>,
    ) -> Result<(), BufferError> {
        self.write_string_ref(&topic_filter.filter)?;
        if sub {
            self.write_u8(topic_filter.sub_options)?;
        }
        Ok(())
    }

    /// Writes the topic filter Vec to the buffer. If the `sub` option is set to `false`, it will not
    /// write the `sub_options` only topic names.
    pub fn write_topic_filters_ref<const MAX: usize>(
        &mut self,
        sub: bool,
        _len: usize,
        filters: &Vec<TopicFilter<'a>, MAX>,
    ) -> Result<(), BufferError> {
        for filter in filters {
            self.write_topic_filter_ref(sub, filter)?;
        }
        Ok(())
    }
}
