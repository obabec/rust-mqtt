use crate::{
    client::raw::NetStateError,
    eio::{self, ErrorKind},
    packet::{RxError, TxError},
    types::ReasonCode,
};

/// The main error returned by `Raw`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<B> {
    /// A packet was too long to encode its length with the variable byte integer
    PacketTooLong,

    /// The underlying Read/Write method returned an error.
    Network(ErrorKind),

    /// The network is in a faulty state.
    Disconnected,

    /// A buffer provision by the `BufferProvider` failed.
    Alloc(B),

    /// A generic constant such as `MAX_PROPERTIES` is too small.
    ConstSpace,

    /// Malformed packet or Protocol Error.
    Server,
}

impl<E: eio::Error, B> From<TxError<E>> for Error<B> {
    fn from(e: TxError<E>) -> Self {
        match e {
            TxError::WriteZero => Self::Network(ErrorKind::WriteZero),
            TxError::Write(e) => Self::Network(e.kind()),
        }
    }
}

impl<E: eio::Error, B> From<RxError<E, B>> for (Error<B>, Option<ReasonCode>) {
    fn from(e: RxError<E, B>) -> Self {
        match e {
            RxError::Read(e) => (Error::Network(e.kind()), None),
            RxError::Buffer(b) => (
                Error::Alloc(b),
                Some(ReasonCode::ImplementationSpecificError),
            ),
            RxError::InsufficientConstSpace => (
                Error::ConstSpace,
                Some(ReasonCode::ImplementationSpecificError),
            ),
            RxError::UnexpectedEOF => (Error::Network(ErrorKind::NotConnected), None),
            RxError::MalformedPacket => (Error::Server, Some(ReasonCode::MalformedPacket)),
            RxError::ProtocolError => (Error::Server, Some(ReasonCode::ProtocolError)),
        }
    }
}

impl<B> From<NetStateError> for Error<B> {
    fn from(_: NetStateError) -> Self {
        Self::Disconnected
    }
}
