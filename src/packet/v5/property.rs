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

use crate::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;
use crate::utils::types::{BinaryData, BufferError, EncodedString, StringPair};

#[derive(Debug, Clone)]
pub enum Property<'a> {
    PayloadFormat(u8),
    MessageExpiryInterval(u32),
    ContentType(EncodedString<'a>),
    ResponseTopic(EncodedString<'a>),
    CorrelationData(BinaryData<'a>),
    SubscriptionIdentifier(u32),
    SessionExpiryInterval(u32),
    AssignedClientIdentifier(EncodedString<'a>),
    ServerKeepAlive(u16),
    AuthenticationMethod(EncodedString<'a>),
    AuthenticationData(BinaryData<'a>),
    RequestProblemInformation(u8),
    WillDelayInterval(u32),
    RequestResponseInformation(u8),
    ResponseInformation(EncodedString<'a>),
    ServerReference(EncodedString<'a>),
    ReasonString(EncodedString<'a>),
    ReceiveMaximum(u16),
    TopicAliasMaximum(u16),
    TopicAlias(u16),
    MaximumQoS(u8),
    RetainAvailable(u8),
    UserProperty(StringPair<'a>),
    MaximumPacketSize(u32),
    WildcardSubscriptionAvailable(u8),
    SubscriptionIdentifierAvailable(u8),
    SharedSubscriptionAvailable(u8),
    Reserved(),
}

impl<'a> Property<'a> {
    pub fn connect_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::SessionExpiryInterval(_u) => true,
            Property::ReceiveMaximum(_u) => true,
            Property::MaximumPacketSize(_u) => true,
            Property::TopicAliasMaximum(_u) => true,
            Property::RequestResponseInformation(_u) => true,
            Property::RequestProblemInformation(_u) => true,
            Property::UserProperty(_u) => true,
            Property::AuthenticationMethod(_u) => true,
            Property::AuthenticationData(_u) => true,
            _ => false,
        }
    }

    pub fn connack_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::SessionExpiryInterval(_u) => true,
            Property::ReceiveMaximum(_u) => true,
            Property::MaximumQoS(_u) => true,
            Property::MaximumPacketSize(_u) => true,
            Property::AssignedClientIdentifier(_u) => true,
            Property::TopicAliasMaximum(_u) => true,
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            Property::WildcardSubscriptionAvailable(_u) => true,
            Property::SubscriptionIdentifierAvailable(_u) => true,
            Property::SharedSubscriptionAvailable(_u) => true,
            Property::ServerKeepAlive(_u) => true,
            Property::ResponseInformation(_u) => true,
            Property::ServerReference(_u) => true,
            Property::AuthenticationMethod(_u) => true,
            Property::AuthenticationData(_u) => true,
            _ => false,
        }
    }

    pub fn publish_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::PayloadFormat(_u) => true,
            Property::MessageExpiryInterval(_u) => true,
            Property::TopicAlias(_u) => true,
            Property::ResponseTopic(_u) => true,
            Property::CorrelationData(_u) => true,
            Property::UserProperty(_u) => true,
            Property::SubscriptionIdentifier(_u) => true,
            Property::ContentType(_u) => true,
            _ => false,
        }
    }

    pub fn puback_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn pubrec_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn pubrel_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn pubcomp_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn subscribe_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::SubscriptionIdentifier(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn suback_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn unsubscribe_property(&self) -> bool {
        matches!(self, Property::UserProperty(_u))
    }

    pub fn unsuback_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn pingreq_property(&self) -> bool {
        warn!("pingreq property list is incomplete");
        false
    }

    pub fn pingresp_property(&self) -> bool {
        warn!("pingresp property list is incomplete");
        false
    }

    pub fn disconnect_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::SessionExpiryInterval(_u) => true,
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            Property::ServerReference(_u) => true,
            _ => false,
        }
    }

    pub fn auth_property(&self) -> bool {
        // not possible to use with associated values with different types
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Property::AuthenticationMethod(_u) => true,
            Property::AuthenticationData(_u) => true,
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        }
    }

    pub fn encoded_len(&self) -> u16 {
        match self {
            Property::PayloadFormat(_u) => 1,
            Property::MessageExpiryInterval(_u) => 4,
            Property::ContentType(u) => u.encoded_len(),
            Property::ResponseTopic(u) => u.encoded_len(),
            Property::CorrelationData(u) => u.encoded_len(),
            Property::SubscriptionIdentifier(u) => {
                VariableByteIntegerEncoder::len(VariableByteIntegerEncoder::encode(*u).unwrap())
                    as u16
            }
            Property::SessionExpiryInterval(_u) => 4,
            Property::AssignedClientIdentifier(u) => u.encoded_len(),
            Property::ServerKeepAlive(_u) => 2,
            Property::AuthenticationMethod(u) => u.encoded_len(),
            Property::AuthenticationData(u) => u.encoded_len(),
            Property::RequestProblemInformation(_u) => 1,
            Property::WillDelayInterval(_u) => 4,
            Property::RequestResponseInformation(_u) => 1,
            Property::ResponseInformation(u) => u.encoded_len(),
            Property::ServerReference(u) => u.encoded_len(),
            Property::ReasonString(u) => u.encoded_len(),
            Property::ReceiveMaximum(_u) => 2,
            Property::TopicAliasMaximum(_u) => 2,
            Property::TopicAlias(_u) => 2,
            Property::MaximumQoS(_u) => 1,
            Property::RetainAvailable(_u) => 1,
            Property::UserProperty(u) => u.encoded_len(),
            Property::MaximumPacketSize(_u) => 4,
            Property::WildcardSubscriptionAvailable(_u) => 1,
            Property::SubscriptionIdentifierAvailable(_u) => 1,
            Property::SharedSubscriptionAvailable(_u) => 1,
            _ => 0,
        }
    }

    pub fn encode(&self, buff_writer: &mut BuffWriter<'a>) -> Result<(), BufferError> {
        match self {
            Property::PayloadFormat(u) => buff_writer.write_u8(*u),
            Property::MessageExpiryInterval(u) => buff_writer.write_u32(*u),
            Property::ContentType(u) => buff_writer.write_string_ref(u),
            Property::ResponseTopic(u) => buff_writer.write_string_ref(u),
            Property::CorrelationData(u) => buff_writer.write_binary_ref(u),
            Property::SubscriptionIdentifier(u) => buff_writer.write_variable_byte_int(*u),
            Property::SessionExpiryInterval(u) => buff_writer.write_u32(*u),
            Property::AssignedClientIdentifier(u) => buff_writer.write_string_ref(u),
            Property::ServerKeepAlive(u) => buff_writer.write_u16(*u),
            Property::AuthenticationMethod(u) => buff_writer.write_string_ref(u),
            Property::AuthenticationData(u) => buff_writer.write_binary_ref(u),
            Property::RequestProblemInformation(u) => buff_writer.write_u8(*u),
            Property::WillDelayInterval(u) => buff_writer.write_u32(*u),
            Property::RequestResponseInformation(u) => buff_writer.write_u8(*u),
            Property::ResponseInformation(u) => buff_writer.write_string_ref(u),
            Property::ServerReference(u) => buff_writer.write_string_ref(u),
            Property::ReasonString(u) => buff_writer.write_string_ref(u),
            Property::ReceiveMaximum(u) => buff_writer.write_u16(*u),
            Property::TopicAliasMaximum(u) => buff_writer.write_u16(*u),
            Property::TopicAlias(u) => buff_writer.write_u16(*u),
            Property::MaximumQoS(u) => buff_writer.write_u8(*u),
            Property::RetainAvailable(u) => buff_writer.write_u8(*u),
            Property::UserProperty(u) => buff_writer.write_string_pair_ref(u),
            Property::MaximumPacketSize(u) => buff_writer.write_u32(*u),
            Property::WildcardSubscriptionAvailable(u) => buff_writer.write_u8(*u),
            Property::SubscriptionIdentifierAvailable(u) => buff_writer.write_u8(*u),
            Property::SharedSubscriptionAvailable(u) => buff_writer.write_u8(*u),
            _ => Err(BufferError::PropertyNotFound),
        }
    }

    pub fn decode(buff_reader: &mut BuffReader<'a>) -> Result<Property<'a>, BufferError> {
        let property_identifier = buff_reader.read_u8();
        return match property_identifier {
            Ok(0x01) => Ok(Property::PayloadFormat(buff_reader.read_u8()?)),
            Ok(0x02) => Ok(Property::MessageExpiryInterval(buff_reader.read_u32()?)),
            Ok(0x03) => Ok(Property::ContentType(buff_reader.read_string()?)),
            Ok(0x08) => Ok(Property::ResponseTopic(buff_reader.read_string()?)),
            Ok(0x09) => Ok(Property::CorrelationData(buff_reader.read_binary()?)),
            Ok(0x0B) => Ok(Property::SubscriptionIdentifier(
                buff_reader.read_variable_byte_int()?,
            )),
            Ok(0x11) => Ok(Property::SessionExpiryInterval(buff_reader.read_u32()?)),
            Ok(0x12) => Ok(Property::AssignedClientIdentifier(
                buff_reader.read_string()?,
            )),
            Ok(0x13) => Ok(Property::ServerKeepAlive(buff_reader.read_u16()?)),
            Ok(0x15) => Ok(Property::AuthenticationMethod(buff_reader.read_string()?)),
            Ok(0x16) => Ok(Property::AuthenticationData(buff_reader.read_binary()?)),
            Ok(0x17) => Ok(Property::RequestProblemInformation(buff_reader.read_u8()?)),
            Ok(0x18) => Ok(Property::WillDelayInterval(buff_reader.read_u32()?)),
            Ok(0x19) => Ok(Property::RequestResponseInformation(buff_reader.read_u8()?)),
            Ok(0x1A) => Ok(Property::ResponseInformation(buff_reader.read_string()?)),
            Ok(0x1C) => Ok(Property::ServerReference(buff_reader.read_string()?)),
            Ok(0x1F) => Ok(Property::ReasonString(buff_reader.read_string()?)),
            Ok(0x21) => Ok(Property::ReceiveMaximum(buff_reader.read_u16()?)),
            Ok(0x22) => Ok(Property::TopicAliasMaximum(buff_reader.read_u16()?)),
            Ok(0x23) => Ok(Property::TopicAlias(buff_reader.read_u16()?)),
            Ok(0x24) => Ok(Property::MaximumQoS(buff_reader.read_u8()?)),
            Ok(0x25) => Ok(Property::RetainAvailable(buff_reader.read_u8()?)),
            Ok(0x26) => Ok(Property::UserProperty(buff_reader.read_string_pair()?)),
            Ok(0x27) => Ok(Property::MaximumPacketSize(buff_reader.read_u32()?)),
            Ok(0x28) => Ok(Property::WildcardSubscriptionAvailable(
                buff_reader.read_u8()?,
            )),
            Ok(0x29) => Ok(Property::SubscriptionIdentifierAvailable(
                buff_reader.read_u8()?,
            )),
            Ok(0x2A) => Ok(Property::SharedSubscriptionAvailable(
                buff_reader.read_u8()?,
            )),
            Err(err) => Err(err),
            _ => Err(BufferError::IdNotFound),
        };
    }
}

impl<'a> From<&Property<'a>> for u8 {
    fn from(value: &Property<'a>) -> Self {
        match value {
            Property::PayloadFormat(_u) => 0x01,
            Property::MessageExpiryInterval(_u) => 0x02,
            Property::ContentType(_u) => 0x03,
            Property::ResponseTopic(_u) => 0x08,
            Property::CorrelationData(_u) => 0x09,
            Property::SubscriptionIdentifier(_u) => 0x0B,
            Property::SessionExpiryInterval(_u) => 0x11,
            Property::AssignedClientIdentifier(_u) => 0x12,
            Property::ServerKeepAlive(_u) => 0x13,
            Property::AuthenticationMethod(_u) => 0x15,
            Property::AuthenticationData(_u) => 0x16,
            Property::RequestProblemInformation(_u) => 0x17,
            Property::WillDelayInterval(_u) => 0x18,
            Property::RequestResponseInformation(_u) => 0x19,
            Property::ResponseInformation(_u) => 0x1A,
            Property::ServerReference(_u) => 0x1C,
            Property::ReasonString(_u) => 0x1F,
            Property::ReceiveMaximum(_u) => 0x21,
            Property::TopicAliasMaximum(_u) => 0x22,
            Property::TopicAlias(_u) => 0x23,
            Property::MaximumQoS(_u) => 0x24,
            Property::RetainAvailable(_u) => 0x25,
            Property::UserProperty(_u) => 0x26,
            Property::MaximumPacketSize(_u) => 0x27,
            Property::WildcardSubscriptionAvailable(_u) => 0x28,
            Property::SubscriptionIdentifierAvailable(_u) => 0x29,
            Property::SharedSubscriptionAvailable(_u) => 0x2A,
            _ => 0x00,
        }
    }
}

impl<'a> From<u8> for Property<'a> {
    fn from(_orig: u8) -> Self {
        warn!("Deserialization of Properties from u8 is not implemented");
        Property::Reserved()
    }
}
