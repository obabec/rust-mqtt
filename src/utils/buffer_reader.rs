use crate::encoding::variable_byte_integer::VariableByteIntegerDecoder;
use core::str;
use core::mem;

#[derive(Debug)]
pub struct EncodedString<'a> {
    pub string: &'a str,
    pub len: u16,
}

impl EncodedString<'_> {
    pub fn new() -> Self {
        Self { string: "", len: 0 }
    }

    pub fn len(&self) -> u16 {
        return self.len + 2;
    }
}

#[derive(Debug)]
pub struct BinaryData<'a> {
    pub bin: &'a [u8],
    pub len: u16
}

impl BinaryData<'_> {
    pub fn new() -> Self {
        Self { bin: &[0], len: 0 }
    }

    pub fn len(&self) -> u16 {
        return self.len + 2;
    }
}

#[derive(Debug)]
pub struct StringPair<'a> {
    pub name: EncodedString<'a>,
    pub value: EncodedString<'a>
}

impl StringPair<'_> {
    pub fn len(&self) -> u16 {
        let ln = self.name.len() + self.value.len();
        return ln;
    }
}

#[derive(Debug)]
pub struct TopicFilter<'a> {
    pub len: u16,
    pub filter: EncodedString<'a>,
    pub sub_options: u8,
}

impl TopicFilter<'_> {
    pub fn new() -> Self {
        Self { len: 0, filter: EncodedString::new(), sub_options: 0 }
    }

    pub fn len(&self) -> u16 {
        return self.len + 2;
    }
}

#[derive(core::fmt::Debug)]
#[derive(Clone)]
pub enum ParseError {
    Utf8Error,
    IndexOutOfBounce,
    VariableByteIntegerError,
    IdNotFound,
    EncodingError,
    DecodingError
}

pub struct BuffReader<'a> {
    buffer: &'a [u8],
    pub position: usize,
}

impl<'a> BuffReader<'a> {
    pub fn incrementPosition(& mut self, increment: usize) {
        self.position = self.position + increment;
    }

    pub fn new(buffer: &'a [u8]) -> Self {
        return BuffReader { buffer: buffer, position: 0 };
    }

    pub fn readVariableByteInt(& mut self) -> Result<u32, ParseError> { 
        let variable_byte_integer: [u8; 4] = [self.buffer[self.position], self.buffer[self.position + 1], self.buffer[self.position + 2], self.buffer[self.position + 3]];
        let mut len: usize = 1;
        if variable_byte_integer[0] & 0x80 == 1 { 
            len = len + 1; 
            if variable_byte_integer[1] & 0x80 == 1 { 
                len = len + 1; 
                if variable_byte_integer[2] & 0x80 == 1 { 
                    len = len + 1; 
                }
            }
        }
        self.incrementPosition(len);
        return VariableByteIntegerDecoder::decode(variable_byte_integer);
    }
    
    pub fn readU32(& mut self) -> Result<u32, ParseError> {
        let (int_bytes, rest) = self.buffer[self.position..].split_at(mem::size_of::<u32>());
        let ret: u32 = u32::from_be_bytes(int_bytes.try_into().unwrap());
        //let ret: u32 = (((self.buffer[self.position] as u32) << 24) | ((self.buffer[self.position + 1] as u32) << 16) | ((self.buffer[self.position + 2] as u32) << 8) | (self.buffer[self.position + 3] as u32)) as u32;
        self.incrementPosition(4);
        return Ok(ret);
    }
    
    pub fn readU16(& mut self) -> Result<u16, ParseError> {
        let (int_bytes, rest) = self.buffer[self.position..].split_at(mem::size_of::<u16>());
        let ret: u16 =  u16::from_be_bytes(int_bytes.try_into().unwrap());
        //(((self.buffer[self.position] as u16) << 8) | (self.buffer[self.position + 1] as u16)) as u16;
        self.incrementPosition(2);
        return Ok(ret);
    }
    
    pub fn readU8(& mut self) -> Result<u8, ParseError> {
        let ret: u8 = self.buffer[self.position];
        self.incrementPosition(1);
        return Ok(ret);
    }
    
    pub fn readString(& mut self) -> Result<EncodedString<'a>, ParseError> {
        let len = self.readU16();
        match len {
            Err(err) => return Err(err),
            _ => log::debug!("[parseString] let not parsed")
        }
        let len_res = len.unwrap();
        let res_str = str::from_utf8(&(self.buffer[self.position..(self.position + len_res as usize)]));
        if res_str.is_err() {
            log::error!("Could not parse utf-8 string");
            return Err(ParseError::Utf8Error);
        }
        return Ok(EncodedString { string: res_str.unwrap(), len: len_res });
    }
    
    //TODO: Index out of bounce err !!!!!
    pub fn readBinary(& mut self) -> Result<BinaryData<'a>, ParseError> {
        let len = self.readU16();
        match len {
            Err(err) => return Err(err),
            _ => log::debug!("[parseBinary] let not parsed")
        }
        let len_res = len.unwrap();
        let res_bin = &(self.buffer[self.position..(self.position + len_res as usize)]);
        return Ok(BinaryData { bin: res_bin, len: len_res });
    }
    
    pub fn readStringPair(& mut self) -> Result<StringPair<'a>, ParseError> {
        let name = self.readString();
        match name {
            Err(err) => return Err(err),
            _ => log::debug!("[String pair] name not parsed")
        }
        let value = self.readString();
        match value {
            Err(err) => return Err(err),
            _ => log::debug!("[String pair] value not parsed")
        }
        return Ok(StringPair { name: name.unwrap(), value: value.unwrap() });
    }

    pub fn readMessage(& mut self) -> &'a [u8] {
        return &self.buffer[self.position..];
    }
}