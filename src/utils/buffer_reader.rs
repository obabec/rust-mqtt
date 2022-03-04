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

use core::mem;
use core::str;

use crate::encoding::variable_byte_integer::VariableByteIntegerDecoder;
use crate::utils::types::{BinaryData, BufferError, EncodedString, StringPair};

/// Buff reader is reading corresponding types from buffer (Byte array) and stores current position
/// (later as cursor)
pub struct BuffReader<'a> {
    buffer: &'a [u8],
    pub position: usize,
    len: usize,
}

impl<'a> BuffReader<'a> {
    pub fn increment_position(&mut self, increment: usize) {
        self.position = self.position + increment;
    }

    pub fn new(buffer: &'a [u8], buff_len: usize) -> Self {
        return BuffReader {
            buffer,
            position: 0,
            len: buff_len,
        };
    }

    /// Variable byte integer can be 1-4 Bytes long. Buffer reader takes all 4 Bytes at first and
    /// than check what is true length of varbyteint and increment cursor by that
    pub fn read_variable_byte_int(&mut self) -> Result<u32, BufferError> {
        let variable_byte_integer: [u8; 4] = [
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ];
        let mut len: usize = 1;
        // Everytime checking first bit of Byte which determines whenever there is continous Byte
        if variable_byte_integer[0] & 0x80 == 1 {
            len = len + 1;
            if variable_byte_integer[1] & 0x80 == 1 {
                len = len + 1;
                if variable_byte_integer[2] & 0x80 == 1 {
                    len = len + 1;
                }
            }
        }
        self.increment_position(len);
        return VariableByteIntegerDecoder::decode(variable_byte_integer);
    }

    /// Reading u32 from buffer as `Big endian`
    pub fn read_u32(&mut self) -> Result<u32, BufferError> {
        let (int_bytes, _rest) = self.buffer[self.position..].split_at(mem::size_of::<u32>());
        let ret: u32 = u32::from_be_bytes(int_bytes.try_into().unwrap());
        self.increment_position(4);
        return Ok(ret);
    }

    /// Reading u16 from buffer as `Big endinan`
    pub fn read_u16(&mut self) -> Result<u16, BufferError> {
        let (int_bytes, _rest) = self.buffer[self.position..].split_at(mem::size_of::<u16>());
        let ret: u16 = u16::from_be_bytes(int_bytes.try_into().unwrap());
        self.increment_position(2);
        return Ok(ret);
    }

    /// Reading one byte from buffer as `Big endian`
    pub fn read_u8(&mut self) -> Result<u8, BufferError> {
        let ret: u8 = self.buffer[self.position];
        self.increment_position(1);
        return Ok(ret);
    }

    /// Reading UTF-8 encoded string from buffer
    pub fn read_string(&mut self) -> Result<EncodedString<'a>, BufferError> {
        let len = self.read_u16();
        match len {
            Err(err) => return Err(err),
            _ => {}
        }
        let len_res = len.unwrap();
        let res_str =
            str::from_utf8(&(self.buffer[self.position..(self.position + len_res as usize)]));
        if res_str.is_err() {
            log::error!("Could not parse utf-8 string");
            return Err(BufferError::Utf8Error);
        }
        self.increment_position(len_res as usize);
        return Ok(EncodedString {
            string: res_str.unwrap(),
            len: len_res,
        });
    }

    /// Read Binary data from buffer
    pub fn read_binary(&mut self) -> Result<BinaryData<'a>, BufferError> {
        let len = self.read_u16()?;
        let res_bin = &(self.buffer[self.position..(self.position + len as usize)]);
        return Ok(BinaryData {
            bin: res_bin,
            len: len,
        });
    }

    /// Read string pair from buffer
    pub fn read_string_pair(&mut self) -> Result<StringPair<'a>, BufferError> {
        let name = self.read_string();
        match name {
            Err(err) => return Err(err),
            _ => log::debug!("[String pair] name not parsed"),
        }
        let value = self.read_string();
        match value {
            Err(err) => return Err(err),
            _ => log::debug!("[String pair] value not parsed"),
        }
        return Ok(StringPair {
            name: name.unwrap(),
            value: value.unwrap(),
        });
    }

    /// Read payload message from buffer
    pub fn read_message(&mut self, total_len: usize) -> &'a [u8] {
        if total_len > self.len {
            return &self.buffer[self.position..self.len];
        }
        return &self.buffer[self.position..total_len];
    }
}
