#![crate_name = "doc"]
/// VariableByteIntegerEncoder and VariableByteIntegerDecoder are implemented based on 
/// pseudo code which is introduced in MQTT version 5.0 OASIS standard accesible from
/// https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901107

/// Variable byte integer encoder structure is help structure which implements function used to
/// encode integer into MQTT variable byte integer format. This format is mainly used to encode
/// lenghts stored in a packet.
pub struct VariableByteIntegerEncoder;
/// Variable byte integers error enumeration is used by both encoder and decoder for
/// error notification.
#[derive(core::fmt::Debug)]
#[derive(Clone)]
pub enum VariableByteIntegerError {
    EncodingError,
    DecodingError
}

pub type VariableByteInteger = [u8; 4];

impl VariableByteIntegerEncoder {
    /// Encode function takes as parameter integer as u32 type and encodes
    /// this integer into maximal 4 Bytes. MSb of each Byte is controll bit.
    /// This bit is saying if there is continuing Byte in stream or not, this way
    /// we can effectively use 1 to 4 Bytes based in integer len.
    pub fn encode(mut target: u32) -> Result<VariableByteInteger, VariableByteIntegerError> {
        // General known informations from OASIS
        const MAX_ENCODABLE: u32 = 268435455;
        const MOD: u32 = 128;
        if target > MAX_ENCODABLE {
            log::error!("Maximal value of integer for encoding was exceeded");
            return Err(VariableByteIntegerError::EncodingError);
        }

        let mut res: [u8; 4] = [0; 4];
        let mut encoded_byte: u8;
        let mut i: usize = 0;

        loop {
            encoded_byte = (target % MOD) as u8;
            target = target / 128;
            if target > 0 {
                encoded_byte = encoded_byte | 128;
            }
            res[i] = encoded_byte;
            i = i + 1;
            if target <= 0 {
                break;
            }
        }
        return Ok(res); 
    }
}

/// Variable byte integer decoder structure is help structure which implements function used to
/// decode message lenghts in MQTT packet and other parts encoded into variable byte integer.
pub struct VariableByteIntegerDecoder;


impl VariableByteIntegerDecoder {
    /// Decode function takes as paramater encoded integer represented
    /// as array of 4 unsigned numbers of exactly 1 Byte each -> 4 Bytes maximal
    /// same as maximal amount of bytes for variable byte encoding in MQTT.
    pub fn decode(encoded: VariableByteInteger) -> Result<u32, VariableByteIntegerError> {
        let mut multiplier: u32 = 1;
        let mut ret: u32 = 0;

        let mut encoded_byte: u8;
        let mut i: usize = 0;
        
        loop {
            encoded_byte = encoded[i];
            i = i + 1;
            ret = ret + ((encoded_byte & 127) as u32 * multiplier) as u32;
            if multiplier > 128 * 128 * 128 {
                return Err(VariableByteIntegerError::DecodingError);
            }
            multiplier = multiplier * 128;
            if (encoded_byte & 128) == 0 {
                break;
            }
        }

        return Ok(ret);
    }
}