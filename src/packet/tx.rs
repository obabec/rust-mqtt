use crate::{
    eio::Write,
    io::{
        err::WriteError,
        write::{Writable, wlen},
    },
    packet::Packet,
    types::VarByteInt,
};

pub trait TxPacket: Packet {
    /// Returns the remaining length of a packet.
    /// This is the value which will ultimately be encoded as the remaining length
    /// in the fixed header.
    fn remaining_len(&self) -> VarByteInt;
    
    /// Returns the full length of the packet from the first byte of the fixed header
    /// to the last byte of the payload.
    fn encoded_len(&self) -> usize {
        let l = self.remaining_len();
        // (type + flags) + remaining length encoded + remaining length
        wlen!(u8) + l.written_len() + l.size()
    }
    
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>>;
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TxError<E> {
    Write(E),
    WriteZero,
}

impl<E> From<WriteError<E>> for TxError<E> {
    fn from(e: WriteError<E>) -> Self {
        match e {
            WriteError::Write(e) => Self::Write(e),
            WriteError::WriteZero => Self::WriteZero,
        }
    }
}
