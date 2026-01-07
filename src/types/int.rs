use crate::{
    eio::{Read, Write},
    fmt::unreachable,
    io::{
        err::{ReadError, WriteError},
        read::Readable,
        write::Writable,
    },
    types::TooLargeToEncode,
};

/// MQTT's variable byte integer encoding. Mainly used for packet length, but also throughout some properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct VarByteInt(u32);

impl<R: Read> Readable<R> for VarByteInt {
    async fn read(net: &mut R) -> Result<Self, ReadError<R::Error>> {
        let mut multiplier = 1;
        let mut value = 0;

        loop {
            let byte = u8::read(net).await?;

            value += (byte & 0x7F) as u32 * multiplier;
            if multiplier > 128 * 128 * 128 {
                return Err(ReadError::MalformedPacket);
            }
            multiplier *= 128;
            if byte & 128 == 0 {
                break;
            }
        }

        Ok(Self(value))
    }
}

impl Writable for VarByteInt {
    fn written_len(&self) -> usize {
        match self.0 {
            0..=127 => 1,
            128..=16_383 => 2,
            16_384..=2_097_151 => 3,
            2_097_152..=Self::MAX_ENCODABLE => 4,
            _ => unreachable!(
                "Invariant, never occurs if VarByteInts are generated using From and TryFrom"
            ),
        }
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let mut x = self.0;
        let mut encoded_byte: u8;

        loop {
            encoded_byte = (x % 128) as u8;
            x /= 128;

            if x > 0 {
                encoded_byte |= 128;
            }
            encoded_byte.write(write).await?;

            if x == 0 {
                return Ok(());
            }
        }
    }
}

impl VarByteInt {
    /// The maximum encodable value using the variable byte integer encoding according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901011>.
    pub const MAX_ENCODABLE: u32 = 268_435_455;

    /// Creates a variable byte integer without checking for the `VarByteInt::MAX_ENCODABLE` invariant.
    ///
    /// # Safety
    /// The value parameter is less than or equal to `VarByteInt::MAX_ENCODABLE`
    pub const unsafe fn new_unchecked(value: u32) -> Self {
        Self(value)
    }

    /// Returns the inner value.
    pub const fn value(&self) -> u32 {
        self.0
    }

    /// Returns `Self::value() as usize`
    pub const fn size(&self) -> usize {
        self.0 as usize
    }

    /// Decodes a variable byte integer from a slice.
    ///
    /// # Safety
    /// The slice contains a validly encoded variable byte integer and is not longer than that encoding.
    pub unsafe fn from_slice_unchecked(slice: &[u8]) -> Self {
        let mut multiplier = 1;
        let mut value = 0;

        for b in slice {
            value += (b & 0x7F) as u32 * multiplier;
            multiplier *= 128;
            if b & 128 == 0 {
                break;
            }
        }

        Self(value)
    }
}

impl TryFrom<u32> for VarByteInt {
    type Error = TooLargeToEncode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > Self::MAX_ENCODABLE {
            Err(TooLargeToEncode)
        } else {
            Ok(Self(value))
        }
    }
}
impl From<u16> for VarByteInt {
    fn from(value: u16) -> Self {
        Self(value as u32)
    }
}
impl From<u8> for VarByteInt {
    fn from(value: u8) -> Self {
        Self(value as u32)
    }
}
