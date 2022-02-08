use crate::encoding::variable_byte_integer::VariableByteIntegerDecoder;
use crate::encoding::variable_byte_integer::VariableByteIntegerError;
use core::str;


pub trait Decode<'a> {
    fn decode(input: &'a [u8], offset: &'a mut usize) -> Result<Self, ProperyParseError> where Self: Sized;
}

pub struct EncodedString<'a> {
    pub string: &'a str,
    pub len: u16
}

pub struct BinaryData<'a> {
    pub bin: &'a [u8],
    pub len: u16
}

pub struct StringPair<'a> {
    pub name: EncodedString<'a>,
    pub value: EncodedString<'a>
}

#[derive(core::fmt::Debug)]
pub enum ProperyParseError {
    Utf8Error,
    IndexOutOfBounce,
    VariableByteIntegerError,
    IdNotFound
}

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
}

fn parseVariableByteInt<'a>(input: &'a [u8], offset: & mut usize) -> Result<u32, VariableByteIntegerError> { 
    let variable_byte_integer: [u8; 4] = [input[*offset], input[*offset + 1], input[*offset + 2], input[*offset + 3]];
    *offset = *offset + 4;
    return VariableByteIntegerDecoder::decode(variable_byte_integer);
}

fn parseU32<'a>(input: &'a [u8], offset: & mut usize) -> Result<u32, ProperyParseError> {
    let ret: u32 = (((input[*offset] as u32) << 24) | ((input[*offset + 1] as u32) << 16) | ((input[*offset + 2] as u32) << 8) | (input[*offset + 3] as u32)) as u32;
    *offset = *offset + 4;
    return Ok(ret);
}

fn parseU16<'a>(input: &'a [u8], offset: & mut usize) -> Result<u16, ProperyParseError> {
    let ret: u16 = (((input[*offset] as u16) << 8) | (input[*offset + 1] as u16)) as u16;
    *offset = *offset + 2;
    return Ok(ret);
}

fn parseU8<'a>(input: &'a [u8], offset: & mut usize) -> Result<u8, ProperyParseError> {
    let ret: u8 = input[*offset];
    *offset = *offset + 1;
    return Ok(ret);
}

fn parseString<'a>(input: &'a [u8], offset: & mut usize) -> Result<EncodedString<'a>, ProperyParseError> {
    let len = parseU16(input, offset);
    match len {
        Err(err) => return Err(err),
        _ => log::debug!("[parseString] let not parsed")
    }
    let len_res = len.unwrap();
    let res_str = str::from_utf8(&(input[*offset..(*offset + len_res as usize)]));
    if res_str.is_err() {
        log::error!("Could not parse utf-8 string");
        return Err(ProperyParseError::Utf8Error);
    }
    return Ok(EncodedString { string: res_str.unwrap(), len: len_res });
}

//TODO: Index out of bounce err !!!!!
fn parseBinary<'a>(input: &'a [u8], offset: & mut usize) -> Result<BinaryData<'a>, ProperyParseError> {
    let len = parseU16(input, offset);
    match len {
        Err(err) => return Err(err),
        _ => log::debug!("[parseBinary] let not parsed")
    }
    let len_res = len.unwrap();
    let res_bin = &(input[*offset..(*offset + len_res as usize)]);
    return Ok(BinaryData { bin: res_bin, len: len_res });
}

fn parseStringPair<'a>(input: &'a [u8], offset: & mut usize) -> Result<StringPair<'a>, ProperyParseError> {
    let name = parseString(input, offset);
    match name {
        Err(err) => return Err(err),
        _ => log::debug!("[String pair] name not parsed")
    }
    let value = parseString(input, offset);
    match value {
        Err(err) => return Err(err),
        _ => log::debug!("[String pair] value not parsed")
    }
    return Ok(StringPair { name: name.unwrap(), value: value.unwrap() });
}

/*impl<'a> From<u8> for Property<'a> {
    fn from(orig: u8) -> Self {
        match orig {
            0x01 => return Property::PayloadFormat,
            0x02 => return Property::MessageExpiryInterval,
            0x03 => return Property::ContentType,
            0x08 => return Property::ResponseTopic,
            0x09 => return Property::CorrelationData,
            0x0B => return Property::SubscriptionIdentifier,
            0x11 => return Property::SessionExpiryInterval,
            0x12 => return Property::AssignedClientIdentifier,
            0x13 => return Property::ServerKeepAlive,
            0x15 => return Property::AuthenticationMethod,
            0x16 => return Property::AuthenticationMethod
            _ => return Property::TopicAlias
        }
    }
}*/