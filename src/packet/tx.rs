use crate::{eio::Write, io::err::WriteError, packet::Packet, types::TooLargeToEncode};

pub trait TxPacket: Packet {
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>>;
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TxError<E> {
    Write(E),
    WriteZero,

    /// The remaining length of a packet is too large to be encoded by a variable byte integer
    RemainingLenExceeded,
}

impl<E> From<WriteError<E>> for TxError<E> {
    fn from(e: WriteError<E>) -> Self {
        match e {
            WriteError::Write(e) => Self::Write(e),
            WriteError::WriteZero => Self::WriteZero,
        }
    }
}

impl<E> From<TooLargeToEncode> for TxError<E> {
    fn from(_: TooLargeToEncode) -> Self {
        Self::RemainingLenExceeded
    }
}
