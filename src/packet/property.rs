use crate::encoding::variable_byte_integer::{VariableByteInteger, VariableByteIntegerEncoder};
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::ParseError;
use crate::utils::buffer_reader::StringPair;
use crate::utils::buffer_writer::BuffWriter;

#[derive(Debug)]
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
    pub fn auth_property(&self) -> bool {
        return match self {
            Property::AuthenticationMethod(_u) => true,
            Property::AuthenticationData(_u) => true,
            Property::ReasonString(_u) => true,
            Property::UserProperty(_u) => true,
            _ => false,
        };
    }

    pub fn len(&self) -> u16 {
        return match self {
            Property::PayloadFormat(_u) => 1,
            Property::MessageExpiryInterval(_u) => 4,
            Property::ContentType(u) => u.len(),
            Property::ResponseTopic(u) => u.len(),
            Property::CorrelationData(u) => u.len(),
            Property::SubscriptionIdentifier(_u) => 4,
            Property::SessionExpiryInterval(_u) => 4,
            Property::AssignedClientIdentifier(u) => u.len(),
            Property::ServerKeepAlive(_u) => 2,
            Property::AuthenticationMethod(u) => u.len(),
            Property::AuthenticationData(u) => u.len(),
            Property::RequestProblemInformation(_u) => 1,
            Property::WillDelayInterval(_u) => 4,
            Property::RequestResponseInformation(_u) => 1,
            Property::ResponseInformation(u) => u.len(),
            Property::ServerReference(u) => u.len(),
            Property::ReasonString(u) => u.len(),
            Property::ReceiveMaximum(_u) => 2,
            Property::TopicAliasMaximum(_u) => 2,
            Property::TopicAlias(_u) => 2,
            Property::MaximumQoS(_u) => 1,
            Property::RetainAvailable(_u) => 1,
            Property::UserProperty(u) => u.len(),
            Property::MaximumPacketSize(_u) => 4,
            Property::WildcardSubscriptionAvailable(_u) => 1,
            Property::SubscriptionIdentifierAvailable(_u) => 1,
            Property::SharedSubscriptionAvailable(_u) => 1,
            _ => 0,
        };
    }

    pub fn encode(&self, buff_writer: &mut BuffWriter<'a>) {
        return match self {
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
            _ => (),
        };
    }

    pub fn decode(buff_reader: &mut BuffReader<'a>) -> Result<Property<'a>, ParseError> {
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
            _ => Err(ParseError::IdNotFound),
        };
    }
}

impl Into<u8> for &Property<'a> {
    fn into(self) -> u8 {
        return match &*self {
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
        };
    }
}

impl From<u8> for Property<'a> {
    fn from(_orig: u8) -> Self {
        return match _orig {
            _ => Property::Reserved(),
        };
    }
}
