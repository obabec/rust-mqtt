use core::num::NonZero;

use crate::eio::{Read, Write};

use crate::io::err::{ReadError, WriteError};
use crate::io::read::Readable;
use crate::io::write::{Writable, wlen};

/// A simple wrapper around [`NonZero`]<[`u16`]>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PacketIdentifier(NonZero<u16>);

impl PacketIdentifier {
    pub(crate) const ONE: Self = Self::new(NonZero::new(1).unwrap());

    pub(crate) const fn new(value: NonZero<u16>) -> Self {
        Self(value)
    }

    pub(crate) fn next(self) -> Self {
        NonZero::new(self.0.get().wrapping_add(1))
            .map(Self)
            .unwrap_or(Self::ONE)
    }

    /// Returns the underlying value.
    pub const fn get(self) -> NonZero<u16> {
        self.0
    }

    pub(crate) const fn get_u16(self) -> u16 {
        self.get().get()
    }
}

impl<R: Read> Readable<R> for PacketIdentifier {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        u16::read(read)
            .await
            .map(NonZero::new)?
            .map(Self)
            .ok_or(ReadError::ProtocolError)
    }
}
impl Writable for PacketIdentifier {
    fn written_len(&self) -> usize {
        wlen!(u16)
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.get_u16().write(write).await
    }
}
