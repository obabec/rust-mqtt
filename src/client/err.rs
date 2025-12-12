use crate::{
    client::raw::RawError,
    eio::ErrorKind,
    header::Reserved,
    types::{MqttString, ReasonCode, TooLargeToEncode},
};

/// The main error returned by `Client`.
///
/// Distincts between unrecoverable and recoverable errors.
/// Recoverability in this context refers to whether the current network connection can
/// be used for further communication after the error has occured.
///
/// # Recovery
/// - For unrecoverable errors, `Client::abort` can be called to send an optional DISCONNECT packet if allowed by specification.
///   You can recover the session by calling `Client::recover`
/// - For recoverable errors, follow the error-specific behaviour.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<'e> {
    /// An underlying Read/Write method returned an error.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    Network(ErrorKind),

    /// The remote server did something the client does not understand / does not match the specification.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    Server,

    /// A buffer or generic constant provided by the user was too small to correctly receive a packet.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    ReceiveBuffer,

    /// A buffer provision by the `BufferProvider` failed. Therefore a packet could not be received correctly.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    Alloc,

    /// An AUTH packet header has been received by the client. AUTH packets are not supported by the client.
    /// The client has scheduled a DISCONNECT packet with `ReasonCode::ImplementationSpecificError`.
    /// The packet body has not been decoded.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    AuthPacketReceived,

    /// The client could not connect to the broker or the broker has sent a DISCONNECT packet.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    Disconnect {
        /// The reason code of the causing CONNACK or DISCONNECT packet. If the disconnection is caused
        /// by a CONNACK packet, the reason code ss always erroneous.
        reason: ReasonCode,
        /// The reason string property of the causing CONNACK or DISCONNECT packet if the server included
        /// a reason string.
        reason_string: Option<MqttString<'e>>,
    },

    /// A method tried to send a packet with a packet identifier that is not tracked as in flight.
    ///
    /// Recoverable error. No action has been taken by the client.
    PacketIdentifierNotInFlight,

    /// A packet was too long to encode its length with the variable byte integer.
    ///
    /// This can currently only be returned from `Client::publish` or `Client::republish`
    ///
    /// Recoverable error. No action has been taken by the client.
    PacketMaxLengthExceeded,

    /// An action was rejected because an internal buffer used for tracking session state is full.
    ///
    /// Recoverable error. Try again after an `Event` has been emitted that indicates that buffer might be free again.
    ///
    /// Example:
    ///     `Client::subscribe` returns this error. Wait until an `Event::Suback` is received.
    ///     This clears a spot in the subscribe packet identifiers.
    SessionBuffer,

    /// A publish now would exceed the server's receive maximum and ultimately cause a protocol error.
    ///
    /// Recoverable error. Try again after either an `Event::PublishAcknowledged` or `Event::PublishComplete`  has
    /// been emitted that indicates that buffer might be free again.
    SendQuotaExceeded,

    /// A publish now with the given session expiry interval would cause a protocol error.
    ///
    /// A disconnection was attempted with a session expiry interval change where the session expiry interval in the
    /// CONNECT packet was zero (`SessionExpiryInterval::EndOnDisconnect`) and was greater than zero
    /// (`SessionExpiryInterval::NeverEnd | SessionExpiryInterval::Seconds(_`) in the DISCONNECT packet.
    ///
    /// Recoverable error. Try disconnecting again without an session expiry interval or with a session expiry interval
    /// of zero (`SessionExpiryInterval::EndOnDisconnect`).
    IllegalDisconnectSessionExpiryInterval,

    /// Another unrecoverable error has been returned earlier. The underlying connection is in a state,
    /// in which it refuses/is not able to perform regular communication.
    ///
    /// Unrecoverable error. `Client::abort` should be called.
    RecoveryRequired,
}

impl<'e> Error<'e> {
    /// Returns whether the client can recover from this error without closing the network connection.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::PacketIdentifierNotInFlight)
    }
}

impl<'e> From<Reserved> for Error<'e> {
    fn from(_: Reserved) -> Self {
        Self::Server
    }
}

impl<'e, B> From<RawError<B>> for Error<'e> {
    fn from(e: RawError<B>) -> Self {
        match e {
            RawError::PacketTooLong => Self::PacketMaxLengthExceeded,
            RawError::Disconnected => Self::RecoveryRequired,
            RawError::Network(e) => Self::Network(e),
            RawError::Alloc(_) => Self::Alloc,
            RawError::ConstSpace => Self::ReceiveBuffer,
            RawError::Server => Self::Server,
        }
    }
}

impl<'e> From<TooLargeToEncode> for Error<'e> {
    fn from(_: TooLargeToEncode) -> Self {
        Self::PacketMaxLengthExceeded
    }
}
