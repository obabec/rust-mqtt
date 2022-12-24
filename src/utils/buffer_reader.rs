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
        self.position += increment;
    }

    pub fn new(buffer: &'a [u8], buff_len: usize) -> Self {
        Self {
            buffer,
            position: 0,
            len: buff_len,
        }
    }

    /// Variable byte integer can be 1-4 Bytes long. Buffer reader takes all 4 Bytes at first and
    /// than check what is true length of varbyteint and increment cursor by that
    pub fn read_variable_byte_int(&mut self) -> Result<u32, BufferError> {
        let mut variable_byte_integer: [u8; 4] = [0; 4];
        let mut len: usize = 1;

        // Everytime checking first bit of Byte which determines whenever there is continuous Byte
        let mut x = 0;
        loop {
            if x >= 4 {
                break;
            }
            if self.position + x >= self.len {
                return Err(BufferError::InsufficientBufferSize);
            }
            if self.buffer[self.position + x] & 0x80 != 0 {
                variable_byte_integer[x] = self.buffer[self.position + x];
                len += 1
            } else {
                variable_byte_integer[x] = self.buffer[self.position + x];
                x += 1;
                if x != 4 {
                    loop {
                        variable_byte_integer[x] = 0;
                        x += 1;
                        if x == 4 {
                            break;
                        }
                    }
                    break;
                }
            }
            x += 1;
        }
        self.increment_position(len);
        VariableByteIntegerDecoder::decode(variable_byte_integer)
    }

    /// Reading u32 from buffer as `Big endian`
    pub fn read_u32(&mut self) -> Result<u32, BufferError> {
        if self.position + 4 > self.len {
            return Err(BufferError::InsufficientBufferSize);
        }
        let (int_bytes, _rest) = self.buffer[self.position..].split_at(mem::size_of::<u32>());
        let ret: u32 = u32::from_be_bytes(int_bytes.try_into().unwrap());
        self.increment_position(4);
        Ok(ret)
    }

    /// Reading u16 from buffer as `Big endinan`
    pub fn read_u16(&mut self) -> Result<u16, BufferError> {
        if self.position + 2 > self.len {
            return Err(BufferError::InsufficientBufferSize);
        }
        let (int_bytes, _rest) = self.buffer[self.position..].split_at(mem::size_of::<u16>());
        let ret: u16 = u16::from_be_bytes(int_bytes.try_into().unwrap());
        self.increment_position(2);
        Ok(ret)
    }

    /// Reading one byte from buffer as `Big endian`
    pub fn read_u8(&mut self) -> Result<u8, BufferError> {
        if self.position >= self.len {
            return Err(BufferError::InsufficientBufferSize);
        }
        let ret: u8 = self.buffer[self.position];
        self.increment_position(1);
        Ok(ret)
    }

    /// Reading UTF-8 encoded string from buffer
    pub fn read_string(&mut self) -> Result<EncodedString<'a>, BufferError> {
        let len = self.read_u16()? as usize;

        if self.position + len > self.len {
            return Err(BufferError::InsufficientBufferSize);
        }

        let res_str = str::from_utf8(&(self.buffer[self.position..(self.position + len)]));
        if res_str.is_err() {
            error!("Could not parse utf-8 string");
            return Err(BufferError::Utf8Error);
        }
        self.increment_position(len);
        Ok(EncodedString {
            string: res_str.unwrap(),
            len: len as u16,
        })
    }

    /// Read Binary data from buffer
    pub fn read_binary(&mut self) -> Result<BinaryData<'a>, BufferError> {
        let len = self.read_u16()?;

        if self.position + len as usize > self.len {
            return Err(BufferError::InsufficientBufferSize);
        }

        let res_bin = &(self.buffer[self.position..(self.position + len as usize)]);
        Ok(BinaryData { bin: res_bin, len })
    }

    /// Read string pair from buffer
    pub fn read_string_pair(&mut self) -> Result<StringPair<'a>, BufferError> {
        let name = self.read_string()?;
        let value = self.read_string()?;
        Ok(StringPair { name, value })
    }

    /// Read payload message from buffer
    pub fn read_message(&mut self, total_len: usize) -> &'a [u8] {
        if total_len > self.len {
            return &self.buffer[self.position..self.len];
        }
        &self.buffer[self.position..total_len]
    }

    /// Peeking (without incremental internal pointer) one byte from buffer as `Big endian`
    pub fn peek_u8(&self) -> Result<u8, BufferError> {
        if self.position >= self.len {
            return Err(BufferError::InsufficientBufferSize);
        }
        Ok(self.buffer[self.position])
    }
}
