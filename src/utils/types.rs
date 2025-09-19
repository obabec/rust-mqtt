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

use crate::utils::types::QualityOfService::{QoS0, QoS1, QoS2, INVALID};

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

/// Topic filter serves as public interface for topic selection and subscription options for `SUBSCRIPTION` packet
#[derive(Clone, Copy, Debug)]
pub struct Topic<'a> {
    pub topic_name: &'a str,
    pub retain_handling: RetainHandling,
    pub retain_as_published: bool,
    pub no_local: bool,
    pub qos: QualityOfService,
}

impl<'a> Topic<'a> {
    pub fn new_with_default_options(topic_name: &'a str, qos: QualityOfService) -> Self {
        Self {
            topic_name,
            retain_handling: RetainHandling::default(),
            retain_as_published: bool::default(),
            no_local: bool::default(),
            qos,
        }
    }
}

impl<'a> From<Topic<'a>> for TopicFilter<'a> {
    fn from(value: Topic<'a>) -> Self {
        let retain_handling_bits = match value.retain_handling {
            RetainHandling::AlwaysSend => 0x00,
            RetainHandling::SendIfNotSubscribedBefore => 0x10,
            RetainHandling::NeverSend => 0x20,
        };

        let retain_as_published_bit = match value.retain_as_published {
            true => 0x08,
            false => 0x00,
        };

        let no_local_bit = match value.no_local {
            true => 0x04,
            false => 0x00,
        };

        let qos_bits = value.qos.into_subscribe_bits();

        let subscribe_options_bits =
            retain_handling_bits | retain_as_published_bit | no_local_bit | qos_bits;

        let mut filter = EncodedString::new();
        filter.string = value.topic_name;
        filter.len = value.topic_name.len() as u16;

        Self {
            filter,
            sub_options: subscribe_options_bits,
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub enum RetainHandling {
    #[default]
    AlwaysSend = 0,
    SendIfNotSubscribedBefore = 1,
    NeverSend = 2,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum QualityOfService {
    QoS0 = 0,
    QoS1 = 1,
    QoS2 = 2,
    INVALID = 3,
}

impl QualityOfService {
    pub fn into_publish_bits(&self) -> u8 {
        match self {
            QoS0 => 0x00,
            QoS1 => 0x02,
            QoS2 => 0x04,
            INVALID => 0x06,
        }
    }

    pub fn into_subscribe_bits(&self) -> u8 {
        match self {
            QoS0 => 0x00,
            QoS1 => 0x01,
            QoS2 => 0x02,
            INVALID => 0x03,
        }
    }

    pub fn from_publish_fixed_header(bits: u8) -> Self {
        let qos_bits = bits & 0x06;
        match qos_bits {
            0x00 => QoS0,
            0x02 => QoS1,
            0x04 => QoS2,
            _ => INVALID,
        }
    }

    pub fn from_subscribe_options(bits: u8) -> Self {
        let qos_bits = bits & 0x03;
        match qos_bits {
            0x00 => QoS0,
            0x01 => QoS1,
            0x02 => QoS2,
            _ => INVALID,
        }
    }

    pub fn from_suback_reason_code(bits: u8) -> Self {
        Self::from_subscribe_options(bits)
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
