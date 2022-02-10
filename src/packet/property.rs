use crate::utils::buffer_reader::ParseError;
use crate::utils::buffer_reader::StringPair;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::utils::buffer_reader::BuffReader;

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
    SharedSubscriptionAvailable(u8)
}

impl<'a> Property<'a> {
    pub fn len(&self) -> u16 {
        match self {
            Property::PayloadFormat(u) => return 1,
            Property::MessageExpiryInterval(u) => return 4,
            Property::ContentType(u) => return u.len(),
            Property::ResponseTopic(u) => return u.len(),
            Property::CorrelationData(u) => return u.len(),
            Property::SubscriptionIdentifier(u) => return 4,
            Property::AssignedClientIdentifier(u) => return u.len(),
            Property::ServerKeepAlive(u) => return 2,
            Property::AuthenticationMethod(u) => return u.len(),
            Property::AuthenticationData(u) => return u.len(),
            Property::RequestProblemInformation(u) => return 1,
            Property::WillDelayInterval(u) => return 4,
            Property::RequestResponseInformation(u) => return 1,
            Property::ResponseInformation(u) => return u.len(),
            Property::ServerReference(u) => return u.len(),
            Property::ReasonString(u) => return u.len(),
            Property::ReceiveMaximum(u) => return 2,
            Property::TopicAliasMaximum(u) => return 2,
            Property::TopicAlias(u) => return 2,
            Property::MaximumQoS(u) => return 1,
            Property::RetainAvailable(u) => return 1,
            Property::UserProperty(u) => return u.len(),
            Property::MaximumPacketSize(u) => return 4,
            Property::WildcardSubscriptionAvailable(u) => return 1,
            Property::SubscriptionIdentifierAvailable(u) => return 1,
            Property::SharedSubscriptionAvailable(u) => return 1,
            _ => return 0
        }
    }

    pub fn decode(buff_reader: & mut BuffReader<'a>) -> Result<Property<'a>, ParseError> {
        let propertyIdentifier = buff_reader.readU8(); 
        match propertyIdentifier {
            Ok(0x01) => return Ok(Property::PayloadFormat(buff_reader.readU8()?)),
            Ok(0x02) => return Ok(Property::MessageExpiryInterval(buff_reader.readU32()?)),
            Ok(0x03) => return Ok(Property::ContentType(buff_reader.readString()?)),
            Ok(0x08) => return Ok(Property::ResponseTopic(buff_reader.readString()?)),
            Ok(0x09) => return Ok(Property::CorrelationData(buff_reader.readBinary()?)),
            Ok(0x0B) => return Ok(Property::SubscriptionIdentifier(buff_reader.readVariableByteInt()?)),
            Ok(0x11) => return Ok(Property::SessionExpiryInterval(buff_reader.readU32()?)),
            Ok(0x12) => return Ok(Property::AssignedClientIdentifier(buff_reader.readString()?)),
            Ok(0x13) => return Ok(Property::ServerKeepAlive(buff_reader.readU16()?)),
            Ok(0x15) => return Ok(Property::AuthenticationMethod(buff_reader.readString()?)),
            Ok(0x16) => return Ok(Property::AuthenticationData(buff_reader.readBinary()?)),
            Ok(0x17) => return Ok(Property::RequestProblemInformation(buff_reader.readU8()?)),
            Ok(0x18) => return Ok(Property::WillDelayInterval(buff_reader.readU32()?)),
            Ok(0x19) => return Ok(Property::RequestResponseInformation(buff_reader.readU8()?)),
            Ok(0x1A) => return Ok(Property::ResponseInformation(buff_reader.readString()?)),
            Ok(0x1C) => return Ok(Property::ServerReference(buff_reader.readString()?)),
            Ok(0x1F) => return Ok(Property::ReasonString(buff_reader.readString()?)),
            Ok(0x21) => return Ok(Property::ReceiveMaximum(buff_reader.readU16()?)),
            Ok(0x22) => return Ok(Property::TopicAliasMaximum(buff_reader.readU16()?)),
            Ok(0x23) => return Ok(Property::TopicAlias(buff_reader.readU16()?)),
            Ok(0x24) => return Ok(Property::MaximumQoS(buff_reader.readU8()?)),
            Ok(0x25) => return Ok(Property::RetainAvailable(buff_reader.readU8()?)),
            Ok(0x26) => return Ok(Property::UserProperty(buff_reader.readStringPair()?)),
            Ok(0x28) => return Ok(Property::WildcardSubscriptionAvailable(buff_reader.readU8()?)),
            Ok(0x29) => return Ok(Property::SubscriptionIdentifierAvailable(buff_reader.readU8()?)),
            Ok(0x2A) => return Ok(Property::SharedSubscriptionAvailable(buff_reader.readU8()?)),
            Err(err) => return Err(err),
            _ => return Err(ParseError::IdNotFound)
        }
    }
}