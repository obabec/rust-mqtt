use core::fmt;

use crate::{
    buffer::BufferProvider,
    eio::Read,
    header::FixedHeader,
    io::{
        err::{BodyReadError, ReadError},
        read::BodyReader,
    },
    packet::Packet,
    v5::property::AtMostOncePropertyError,
};

pub trait RxPacket<'p>: Packet + Sized {
    /// Receives a packet. Must check the fixed header for correctness.
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>>;
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RxError<E, B> {
    Read(E),
    Buffer(B),

    /// Constant space somewhere is not enough, e.g. `Vec<ReasonCode, MAX_TOPIC_FILTERS>` in SUBACK
    InsufficientConstSpace,

    UnexpectedEOF,

    MalformedPacket,
    ProtocolError,
}

impl<E, B: fmt::Debug> From<BodyReadError<E, B>> for RxError<E, B> {
    fn from(e: BodyReadError<E, B>) -> Self {
        match e {
            BodyReadError::Read(e) => Self::Read(e),
            BodyReadError::Buffer(b) => Self::Buffer(b),
            BodyReadError::UnexpectedEOF => Self::UnexpectedEOF,
            BodyReadError::InsufficientRemainingLen => Self::MalformedPacket,
            BodyReadError::MalformedPacket => Self::MalformedPacket,
            BodyReadError::ProtocolError => Self::ProtocolError,
        }
    }
}
impl<E, B: fmt::Debug> From<ReadError<BodyReadError<E, B>>> for RxError<E, B> {
    fn from(e: ReadError<BodyReadError<E, B>>) -> Self {
        match e {
            ReadError::Read(e) => e.into(),
            ReadError::UnexpectedEOF => Self::UnexpectedEOF,
            ReadError::MalformedPacket => Self::MalformedPacket,
            ReadError::ProtocolError => Self::ProtocolError,
        }
    }
}
impl<E, B: fmt::Debug> From<ReadError<E>> for RxError<E, B> {
    fn from(e: ReadError<E>) -> Self {
        match e {
            ReadError::Read(e) => Self::Read(e),
            ReadError::UnexpectedEOF => Self::UnexpectedEOF,
            ReadError::MalformedPacket => Self::MalformedPacket,
            ReadError::ProtocolError => Self::ProtocolError,
        }
    }
}
impl<E, B: fmt::Debug> From<AtMostOncePropertyError<ReadError<BodyReadError<E, B>>>>
    for RxError<E, B>
{
    fn from(e: AtMostOncePropertyError<ReadError<BodyReadError<E, B>>>) -> Self {
        match e {
            AtMostOncePropertyError::Read(e) => e.into(),
            AtMostOncePropertyError::AlreadySet => Self::ProtocolError,
        }
    }
}
