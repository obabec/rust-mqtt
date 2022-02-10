use crate::utils::buffer_reader::ProperyParseError;
use crate::utils::buffer_reader::StringPair;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::encoding::variable_byte_integer::VariableByteIntegerError;
use crate::utils::buffer_reader::BuffReader;

#[derive(Clone)]
pub enum Property<'a> {
    PayloadFormat(Result<u8, ProperyParseError>),
    MessageExpiryInterval(Result<u32, ProperyParseError>),
    ContentType(Result<EncodedString<'a>, ProperyParseError>),
    ResponseTopic(Result<EncodedString<'a>, ProperyParseError>),
    CorrelationData(Result<BinaryData<'a>, ProperyParseError>),
    SubscriptionIdentifier(Result<u32, VariableByteIntegerError>),
    SessionExpiryInterval(Result<u32, ProperyParseError>),
    AssignedClientIdentifier(Result<EncodedString<'a>, ProperyParseError>),
    ServerKeepAlive(Result<u16, ProperyParseError>),
    AuthenticationMethod(Result<EncodedString<'a>, ProperyParseError>),
    AuthenticationData(Result<BinaryData<'a>, ProperyParseError>),
    RequestProblemInformation(Result<u8, ProperyParseError>),
    WillDelayInterval(Result<u32, ProperyParseError>),
    RequestResponseInformation(Result<u8, ProperyParseError>),
    ResponseInformation(Result<EncodedString<'a>, ProperyParseError>),
    ServerReference(Result<EncodedString<'a>, ProperyParseError>),
    ReasonString(Result<EncodedString<'a>, ProperyParseError>),
    ReceiveMaximum(Result<u16, ProperyParseError>),
    TopicAliasMaximum(Result<u16, ProperyParseError>),
    TopicAlias(Result<u16, ProperyParseError>),
    MaximumQoS(Result<u8, ProperyParseError>),
    RetainAvailable(Result<u8, ProperyParseError>),
    UserProperty(Result<StringPair<'a>, ProperyParseError>),
    MaximumPacketSize(Result<u32, ProperyParseError>),
    WildcardSubscriptionAvailable(Result<u8, ProperyParseError>),
    SubscriptionIdentifierAvailable(Result<u8, ProperyParseError>),
    SharedSubscriptionAvailable(Result<u8, ProperyParseError>)
}

impl<'a> Property<'a> {
    pub fn len(self) -> u16 {
        match self {
            Property::PayloadFormat(u) => return 1,
            Property::MessageExpiryInterval(u) => return 4,
            Property::ContentType(u) => return u.unwrap().len(),
            Property::ResponseTopic(u) => return u.unwrap().len(),
            Property::CorrelationData(u) => return u.unwrap().len(),
            Property::SubscriptionIdentifier(u) => return 4,
            Property::AssignedClientIdentifier(u) => return u.unwrap().len(),
            Property::ServerKeepAlive(u) => return 2,
            Property::AuthenticationMethod(u) => return u.unwrap().len(),
            Property::AuthenticationData(u) => return u.unwrap().len(),
            Property::RequestProblemInformation(u) => return 1,
            Property::WillDelayInterval(u) => return 4,
            Property::RequestResponseInformation(u) => return 1,
            Property::ResponseInformation(u) => return u.unwrap().len(),
            Property::ServerReference(u) => return u.unwrap().len(),
            Property::ReasonString(u) => return u.unwrap().len(),
            Property::ReceiveMaximum(u) => return 2,
            Property::TopicAliasMaximum(u) => return 2,
            Property::TopicAlias(u) => return 2,
            Property::MaximumQoS(u) => return 1,
            Property::RetainAvailable(u) => return 1,
            Property::UserProperty(u) => return u.unwrap().len(),
            Property::MaximumPacketSize(u) => return 4,
            Property::WildcardSubscriptionAvailable(u) => return 1,
            Property::SubscriptionIdentifierAvailable(u) => return 1,
            Property::SharedSubscriptionAvailable(u) => return 1,
            _ => return 0
        }
    }

    pub fn decode(buff_reader: & mut BuffReader<'a>) -> Result<Property<'a>, ProperyParseError> {
        let propertyIdentifier = buff_reader.readU8(); 
        match propertyIdentifier {
            Ok(0x01) => return Ok(Property::PayloadFormat(buff_reader.readU8())),
            Ok(0x02) => return Ok(Property::MessageExpiryInterval(buff_reader.readU32())),
            Ok(0x03) => return Ok(Property::ContentType(buff_reader.readString())),
            Ok(0x08) => return Ok(Property::ResponseTopic(buff_reader.readString())),
            Ok(0x09) => return Ok(Property::CorrelationData(buff_reader.readBinary())),
            Ok(0x0B) => return Ok(Property::SubscriptionIdentifier(buff_reader.readVariableByteInt())),
            Ok(0x11) => return Ok(Property::SessionExpiryInterval(buff_reader.readU32())),
            Ok(0x12) => return Ok(Property::AssignedClientIdentifier(buff_reader.readString())),
            Ok(0x13) => return Ok(Property::ServerKeepAlive(buff_reader.readU16())),
            Ok(0x15) => return Ok(Property::AuthenticationMethod(buff_reader.readString())),
            Ok(0x16) => return Ok(Property::AuthenticationData(buff_reader.readBinary())),
            Ok(0x17) => return Ok(Property::RequestProblemInformation(buff_reader.readU8())),
            Ok(0x18) => return Ok(Property::WillDelayInterval(buff_reader.readU32())),
            Ok(0x19) => return Ok(Property::RequestResponseInformation(buff_reader.readU8())),
            Ok(0x1A) => return Ok(Property::ResponseInformation(buff_reader.readString())),
            Ok(0x1C) => return Ok(Property::ServerReference(buff_reader.readString())),
            Ok(0x1F) => return Ok(Property::ReasonString(buff_reader.readString())),
            Ok(0x21) => return Ok(Property::ReceiveMaximum(buff_reader.readU16())),
            Ok(0x22) => return Ok(Property::TopicAliasMaximum(buff_reader.readU16())),
            Ok(0x23) => return Ok(Property::TopicAlias(buff_reader.readU16())),
            Ok(0x24) => return Ok(Property::MaximumQoS(buff_reader.readU8())),
            Ok(0x25) => return Ok(Property::RetainAvailable(buff_reader.readU8())),
            Ok(0x26) => return Ok(Property::UserProperty(buff_reader.readStringPair())),
            Ok(0x28) => return Ok(Property::WildcardSubscriptionAvailable(buff_reader.readU8())),
            Ok(0x29) => return Ok(Property::SubscriptionIdentifierAvailable(buff_reader.readU8())),
            Ok(0x2A) => return Ok(Property::SharedSubscriptionAvailable(buff_reader.readU8())),
            Err(err) => return Err(err),
            _ => return Err(ProperyParseError::IdNotFound)
        }
    }
}