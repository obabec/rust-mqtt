use core::num::NonZero;

use crate::{
    eio::{Read, Write},
    io::{
        err::{ReadError, WriteError},
        read::Readable,
        write::{Writable, wlen},
    },
};

/// A simple wrapper around [`NonZero`]<[`u16`]>.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PacketIdentifier(NonZero<u16>);

impl PacketIdentifier {
    pub(crate) const ONE: Self = Self::new(NonZero::new(1).unwrap());

    pub(crate) const fn new(value: NonZero<u16>) -> Self {
        Self(value)
    }

    pub(crate) fn next(self) -> Self {
        NonZero::new(self.0.get().wrapping_add(1)).map_or(Self::ONE, Self)
    }

    /// Returns the underlying value.
    #[must_use]
    pub const fn get(self) -> NonZero<u16> {
        self.0
    }

    pub(crate) const fn get_u16(self) -> u16 {
        self.get().get()
    }
}

impl core::fmt::Debug for PacketIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        self.get().fmt(f)
    }
}
impl core::fmt::Display for PacketIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for PacketIdentifier {
    fn format(&self, fmt: defmt::Formatter) {
        self.get().format(fmt)
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
