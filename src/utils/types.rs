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

use core::fmt::{Display, Formatter};

#[derive(core::fmt::Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BufferError {
    Utf8Error,
    InsufficientBufferSize,
    VariableByteIntegerError,
    IdNotFound,
    EncodingError,
    DecodingError,
    PacketTypeMismatch,
    WrongPacketToDecode,
    WrongPacketToEncode,
    PropertyNotFound,
}

impl Display for BufferError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match *self {
            BufferError::Utf8Error => write!(f, "Error encountered during UTF8 decoding!"),
            BufferError::InsufficientBufferSize => write!(f, "Buffer size is not sufficient for packet!"),
            BufferError::VariableByteIntegerError => write!(f, "Error encountered during variable byte integer decoding / encoding!"),
            BufferError::IdNotFound => write!(f, "Packet identifier not found!"),
            BufferError::EncodingError => write!(f, "Error encountered during packet encoding!"),
            BufferError::DecodingError => write!(f, "Error encountered during packet decoding!"),
            BufferError::PacketTypeMismatch => write!(f, "Packet type not matched during decoding (Received different packet type than encode type)!"),
            BufferError::WrongPacketToDecode => write!(f, "Not able to decode packet, this packet is used just for sending to broker, not receiving by client!"),
            BufferError::WrongPacketToEncode => write!(f, "Not able to encode packet, this packet is used only from server to client not the opposite way!"),
            BufferError::PropertyNotFound => write!(f, "Property with ID not found!")
        }
    }
}
/// Encoded string provides structure representing UTF-8 encoded string in MQTTv5 packets
#[derive(Debug, Clone, Default)]
pub struct EncodedString<'a> {
    pub string: &'a str,
    pub len: u16,
}

impl EncodedString<'_> {
    pub fn new() -> Self {
        Self { string: "", len: 0 }
    }

    /// Return length of string
    pub fn encoded_len(&self) -> u16 {
        self.len + 2
    }
}

/// Binary data represents `Binary data` in MQTTv5 protocol
#[derive(Debug, Clone, Default)]
pub struct BinaryData<'a> {
    pub bin: &'a [u8],
    pub len: u16,
}

impl BinaryData<'_> {
    pub fn new() -> Self {
        Self { bin: &[0], len: 0 }
    }
    /// Returns length of Byte array
    pub fn encoded_len(&self) -> u16 {
        self.len + 2
    }
}

/// String pair struct represents `String pair` in MQTTv5 (2 UTF-8 encoded strings name-value)
#[derive(Debug, Clone, Default)]
pub struct StringPair<'a> {
    pub name: EncodedString<'a>,
    pub value: EncodedString<'a>,
}

impl StringPair<'_> {
    pub fn new() -> Self {
        Self {
            name: EncodedString::new(),
            value: EncodedString::new(),
        }
    }
    /// Returns length which is equal to sum of the lenghts of UTF-8 encoded strings in pair
    pub fn encoded_len(&self) -> u16 {
        self.name.encoded_len() + self.value.encoded_len()
    }
}

/// Topic filter serves as bound for topic selection and subscription options for `SUBSCRIPTION` packet
#[derive(Debug, Default)]
pub struct TopicFilter<'a> {
    pub filter: EncodedString<'a>,
    pub sub_options: u8,
}

impl TopicFilter<'_> {
    pub fn new() -> Self {
        Self {
            filter: EncodedString::new(),
            sub_options: 0,
        }
    }

    pub fn encoded_len(&self) -> u16 {
        self.filter.len + 3
    }
}
