use crate::{
    eio::Read,
    fmt::trace,
    header::{FixedHeader, PacketType},
    io::err::ReadError,
    types::VarByteInt,
};

#[derive(Debug)]
pub struct HeaderState {
    buffer: [u8; 5],
    read: u8,
}

impl HeaderState {
    pub fn new() -> Self {
        Self {
            buffer: [0; 5],
            read: 0,
        }
    }

    /// Cancel-safe
    ///
    /// Attempts to complete the header by reading a single byte
    ///
    /// Resets the state before returning
    pub async fn update<R: Read>(
        &mut self,
        r: &mut R,
    ) -> Result<Option<FixedHeader>, ReadError<R::Error>> {
        let next_byte = &mut self.buffer[(self.read as usize)..(self.read as usize + 1)];
        self.read += r.read(next_byte).await? as u8;
        trace!("received {}", next_byte[0]);

        if self.read == 1 {
            return match PacketType::from_type_and_flags(next_byte[0]) {
                Ok(_) => Ok(None),
                Err(_) => Err(ReadError::MalformedPacket),
            };
        }

        let is_continuation_byte = next_byte[0] >= 128;
        if self.read < 5 && is_continuation_byte {
            return Ok(None);
        }
        if self.read == 5 {
            return Err(ReadError::MalformedPacket);
        }

        let slice = &self.buffer[1..(self.read as usize + 1)];

        // Safety: We checked that the slice is within the valid range and
        // that the last byte matches the end condition of the variable byte integer encoding
        let remaining_len = unsafe { VarByteInt::from_slice_unchecked(slice) };

        self.read = 0;
        Ok(Some(FixedHeader {
            type_and_flags: self.buffer[0],
            remaining_len,
        }))
    }
}
