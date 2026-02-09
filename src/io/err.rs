use core::{
    error::Error,
    fmt::{self, Display},
};

use crate::eio::{self, ErrorKind, ReadExactError};

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReadError<E> {
    Read(E),
    UnexpectedEOF,
    MalformedPacket,
    ProtocolError,
    InvalidTopicName,
}

impl<E, B: fmt::Debug> From<BodyReadError<E, B>> for ReadError<BodyReadError<E, B>> {
    fn from(e: BodyReadError<E, B>) -> Self {
        match e {
            e @ BodyReadError::InsufficientRemainingLen => Self::Read(e),
            e @ BodyReadError::Read(_) => Self::Read(e),
            e @ BodyReadError::Buffer(_) => Self::Read(e),
            BodyReadError::UnexpectedEOF => Self::UnexpectedEOF,
            BodyReadError::MalformedPacket => Self::MalformedPacket,
            BodyReadError::ProtocolError => Self::ProtocolError,
            BodyReadError::InvalidTopicName => Self::InvalidTopicName,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BodyReadError<E, B: fmt::Debug> {
    Read(E),

    /// A buffer provision failed.
    Buffer(B),

    /// EOF has been returned by a Read method.
    UnexpectedEOF,

    /// There is not enough `remaining length` to read a packet field
    ///
    /// The difference to UnexpectedEOF is that this can be a boundary set by the programm.
    /// UnexpectedEOF is caused by the underlying Read
    InsufficientRemainingLen,

    MalformedPacket,
    ProtocolError,
    InvalidTopicName,
}
impl<E: fmt::Debug, B: fmt::Debug> Display for BodyReadError<E, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl<E: Error, B: fmt::Debug> Error for BodyReadError<E, B> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read(e) => e.source(),
            Self::Buffer(_) => None,
            Self::UnexpectedEOF => None,
            Self::InsufficientRemainingLen => None,
            Self::MalformedPacket => None,
            Self::ProtocolError => None,
            Self::InvalidTopicName => None,
        }
    }
}
impl<E: eio::Error, B: fmt::Debug> eio::Error for BodyReadError<E, B> {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Read(e) => e.kind(),
            Self::Buffer(_) => ErrorKind::OutOfMemory,
            Self::UnexpectedEOF => ErrorKind::Other,
            Self::InsufficientRemainingLen => ErrorKind::InvalidData,
            Self::MalformedPacket => ErrorKind::InvalidData,
            Self::ProtocolError => ErrorKind::InvalidData,
            Self::InvalidTopicName => ErrorKind::InvalidData,
        }
    }
}

impl<E, B: fmt::Debug> From<E> for BodyReadError<E, B> {
    fn from(e: E) -> Self {
        Self::Read(e)
    }
}
impl<E, B: fmt::Debug> From<ReadExactError<E>> for BodyReadError<E, B> {
    fn from(e: ReadExactError<E>) -> Self {
        match e {
            ReadExactError::UnexpectedEof => Self::UnexpectedEOF,
            ReadExactError::Other(e) => Self::Read(e),
        }
    }
}
impl<E, B: fmt::Debug> From<ReadError<E>> for BodyReadError<E, B> {
    fn from(e: ReadError<E>) -> Self {
        match e {
            ReadError::Read(e) => Self::Read(e),
            ReadError::UnexpectedEOF => Self::UnexpectedEOF,
            ReadError::MalformedPacket => Self::MalformedPacket,
            ReadError::ProtocolError => Self::ProtocolError,
            ReadError::InvalidTopicName => Self::InvalidTopicName,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WriteError<E> {
    WriteZero,
    Write(E),
}

impl<E> From<E> for WriteError<E> {
    fn from(e: E) -> Self {
        Self::Write(e)
    }
}
