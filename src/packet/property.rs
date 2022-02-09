use crate::utils::buffer_reader::ProperyParseError;
use crate::utils::buffer_reader::StringPair;
use crate::utils::buffer_reader::EncodedString;
use crate::utils::buffer_reader::BinaryData;
use crate::encoding::variable_byte_integer::VariableByteIntegerError;
/*use crate::encoding::variable_byte_integer::VariableByteIntegerDecoder;
use crate::encoding::variable_byte_integer::VariableByteIntegerError;
use core::str;


pub trait Decode<'a> {
    fn decode(input: &'a [u8], offset: &'a mut usize) -> Result<Self, ProperyParseError> where Self: Sized;
}
*/
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

/*
impl<'a> Decode<'a> for Property<'a> {
    fn decode(input: &'a [u8], offset: &'a mut usize) -> Result<Self, ProperyParseError> {
        let propertyIdentifier = parseU8(input, offset); 
        match propertyIdentifier {
            Ok(0x01) => return Ok(Property::PayloadFormat(parseU8(input, offset))),
            Ok(0x02) => return Ok(Property::MessageExpiryInterval(parseU32(input, offset))),
            Ok(0x03) => return Ok(Property::ContentType(parseString(input, offset))),
            Ok(0x08) => return Ok(Property::ResponseTopic(parseString(input, offset))),
            Ok(0x09) => return Ok(Property::CorrelationData(parseBinary(input, offset))),
            Ok(0x0B) => return Ok(Property::SubscriptionIdentifier(parseVariableByteInt(input, offset))),
            Ok(0x11) => return Ok(Property::SessionExpiryInterval(parseU32(input, offset))),
            Ok(0x12) => return Ok(Property::AssignedClientIdentifier(parseString(input, offset))),
            Ok(0x13) => return Ok(Property::ServerKeepAlive(parseU16(input, offset))),
            Ok(0x15) => return Ok(Property::AuthenticationMethod(parseString(input, offset))),
            Ok(0x16) => return Ok(Property::AuthenticationData(parseBinary(input, offset))),
            Ok(0x17) => return Ok(Property::RequestProblemInformation(parseU8(input, offset))),
            Ok(0x18) => return Ok(Property::WillDelayInterval(parseU32(input, offset))),
            Ok(0x19) => return Ok(Property::RequestResponseInformation(parseU8(input, offset))),
            Ok(0x1A) => return Ok(Property::ResponseInformation(parseString(input, offset))),
            Ok(0x1C) => return Ok(Property::ServerReference(parseString(input, offset))),
            Ok(0x1F) => return Ok(Property::ReasonString(parseString(input, offset))),
            Ok(0x21) => return Ok(Property::ReceiveMaximum(parseU16(input, offset))),
            Ok(0x22) => return Ok(Property::TopicAliasMaximum(parseU16(input, offset))),
            Ok(0x23) => return Ok(Property::TopicAlias(parseU16(input, offset))),
            Ok(0x24) => return Ok(Property::MaximumQoS(parseU8(input, offset))),
            Ok(0x25) => return Ok(Property::RetainAvailable(parseU8(input, offset))),
            Ok(0x26) => return Ok(Property::UserProperty(parseStringPair(input, offset))),
            Ok(0x28) => return Ok(Property::WildcardSubscriptionAvailable(parseU8(input, offset))),
            Ok(0x29) => return Ok(Property::SubscriptionIdentifierAvailable(parseU8(input, offset))),
            Ok(0x2A) => return Ok(Property::SharedSubscriptionAvailable(parseU8(input, offset))),
            Err(err) => return Err(err),
            _ => return Err(ProperyParseError::IdNotFound)
        }
    }
}*/

