use heapless::Vec;

use crate::{
    client::raw::RawError,
    eio::ErrorKind,
    header::Reserved,
    types::{MqttString, MqttStringPair, ReasonCode, TooLargeToEncode},
};

/// The main error returned by [`crate::client::Client`].
///
/// Distincts between unrecoverable and recoverable errors.
/// Recoverability in this context refers to whether the current network connection can
/// be used for further communication after the error has occured.
///
/// # Recovery
/// - For unrecoverable errors, [`crate::client::Client::abort`] can be called to send an optional DISCONNECT
///   packet if allowed by specification. You can then try to recover the session by calling
///   [`crate::client::Client::connect`] again without clean start.
/// - For recoverable errors, follow the error-specific behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<'e, const MAX_USER_PROPERTIES: usize> {
    /// An underlying Read/Write method returned an error.
    ///
    /// Unrecoverable error. [`crate::client::Client::abort`] should be called.
    Network(ErrorKind),

    /// The remote server did something the client does not understand / does not match the specification.
    ///
    /// Unrecoverable error. [`crate::client::Client::abort`] should be called.
    Server,

    /// A buffer provision by the [`crate::buffer::BufferProvider`] failed. Therefore a packet
    /// could not be received correctly.
    ///
    /// Unrecoverable error. [`crate::client::Client::abort`] should be called.
    Alloc,

    /// An AUTH packet header has been received by the client. AUTH packets are not supported by the client.
    /// The client has scheduled a DISCONNECT packet with [`ReasonCode::ImplementationSpecificError`].
    /// The packet body has not been decoded.
    ///
    /// Unrecoverable error. [`crate::client::Client::abort`] should be called.
    AuthPacketReceived,

    /// The client could not connect to the broker or the broker has sent a DISCONNECT packet.
    ///
    /// Unrecoverable error. [`crate::client::Client::abort`] should be called.
    Disconnect {
        /// The [`ReasonCode`] of the causing CONNACK or DISCONNECT packet. If the disconnection is caused
        /// by a CONNACK packet, the reason code ss always erroneous.
        reason: ReasonCode,

        /// The reason string property of the causing CONNACK or DISCONNECT packet if the server included
        /// a reason string.
        reason_string: Option<MqttString<'e>>,

        /// The user property entries in the causing CONNACK or DISCONNECT packet. If the vector is full,
        /// this list might not be exhaustive.
        user_properties: Vec<MqttStringPair<'e>, MAX_USER_PROPERTIES>,

        /// The server reference property of the causing CONNACK or DISCONNCET packet if the server included
        /// a server reference. Identifies another server which can be used.
        server_reference: Option<MqttString<'e>>,
    },

    /// Another unrecoverable error has been returned earlier. The underlying connection is in a state,
    /// in which it refuses/is not able to perform regular communication.
    ///
    /// Unrecoverable error. [`crate::client::Client::abort`] should be called.
    RecoveryRequired,

    /// A republish of a packet without an in flight entry was attempted.
    ///
    /// Recoverable error. No action has been taken by the client.
    PacketIdentifierNotInFlight,

    /// A republish of a packet with a quality of service that does not match the quality of
    /// service of the original publication was attempted.
    ///
    /// Recoverable error. No action has been taken by the client.
    RepublishQoSNotMatching,

    /// A republish of a packet whose corresponding PUBREL packet has already been sent was attempted.
    /// Sending the PUBLISH packet in this case would result in a protocol violation.
    ///
    /// Recoverable error. No action has been taken by the client.
    PacketIdentifierAwaitingPubcomp,

    /// A packet was too long to encode its length with the variable byte integer.
    ///
    /// This can currently only be returned from [`crate::client::Client::publish`] or
    /// [`crate::client::Client::republish`].
    ///
    /// Recoverable error. No action has been taken by the client.
    PacketMaximumLengthExceeded,

    /// A packet is too long and would exceed the servers maximum packet size.
    ///
    /// Recoverable error. No action has been taken by the client.
    ServerMaximumPacketSizeExceeded,

    /// The value of a topic alias in an outgoing PUBLISH packet was 0 or greater than the server's maximum allowed
    /// value. Sending this PUBLISH packet would raise a protocol error.
    ///
    /// Recoverable error. No action has been taken by the client.
    InvalidTopicAlias,

    /// An action was rejected because an internal buffer used for tracking session state is full.
    ///
    /// Recoverable error. Try again after a [`crate::client::event::Event`] has been emitted that indicates
    /// that buffer might be free again.
    ///
    /// Example:
    ///     [`crate::client::Client::subscribe`] returns this error. Wait until a
    ///     [`crate::client::event::Event::Suback`] is received.
    ///     This clears a spot in the subscribe packet identifiers.
    SessionBuffer,

    /// A publish now would exceed the server's receive maximum and ultimately cause a protocol error.
    ///
    /// Recoverable error. Try again after either [`crate::client::event::Event::PublishAcknowledged`] or
    /// [`crate::client::event::Event::PublishComplete`] has been emitted that indicates that buffer might be
    /// free again.
    /// been emitted that indicates that buffer might be free again.
    SendQuotaExceeded,

    /// An operation was attempted which the server stated it does not support. If the requested operation
    /// were executed as is, a protocol error would be caused.
    /// 
    /// This could be:
    /// - a shared subscription (topic filter starts with "$share") being attempted despite shared
    ///   subscriptions not being available on the server
    /// - a subscription identifier being specified despite subscription identifiers not being available on
    ///   the server
    /// - a wildcard occuring in a topic filter despite wildcard subscriptions not being available on the
    ///   server
    /// - a publication with a quality of service level greater than the server's maximum quality of service
    ///   being attempted
    /// 
    /// Recoverable error. No action has been taken by the client.
    UnsupportedByServer,

    /// A disconnect now with the given session expiry interval would cause a protocol error.
    ///
    /// A disconnection was attempted with a session expiry interval change where the session expiry interval in the
    /// CONNECT packet was zero ([`crate::config::SessionExpiryInterval::EndOnDisconnect`]) and was
    /// greater than zero ([`crate::config::SessionExpiryInterval::NeverEnd`] or
    /// [`crate::config::SessionExpiryInterval::Seconds`]) in the DISCONNECT packet.
    ///
    /// Recoverable error. Try disconnecting again without an session expiry interval or with a
    /// session expiry interval of zero ([`crate::config::SessionExpiryInterval::EndOnDisconnect`]).
    IllegalDisconnectSessionExpiryInterval,
}

impl<const MAX_USER_PROPERTIES: usize> Error<'_, MAX_USER_PROPERTIES> {
    /// Returns whether the client can recover from this error without closing the network connection.
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PacketIdentifierNotInFlight
                | Self::RepublishQoSNotMatching
                | Self::PacketIdentifierAwaitingPubcomp
                | Self::PacketMaximumLengthExceeded
                | Self::ServerMaximumPacketSizeExceeded
                | Self::InvalidTopicAlias
                | Self::SessionBuffer
                | Self::SendQuotaExceeded
                | Self::UnsupportedByServer
                | Self::IllegalDisconnectSessionExpiryInterval
        )
    }
}
impl<'e> Error<'e, 0> {
    /// Converts an [`Error<0>`] into an [`Error<N>`] with any N.
    ///
    /// This cannot be a [`From`] implementation because `From<Error<0>> for Error<N>` would
    /// collide with the blanket implementation `From<T> for T`. The reason this function is
    /// only implemented for `MAX_USER_PROPERTIES` = 0 is to prevent potentially surprisng
    /// panics when converting from more user properties to less.
    pub fn inflate<const MAX_USER_PROPERTIES: usize>(self) -> Error<'e, MAX_USER_PROPERTIES> {
        match self {
            Self::Network(error_kind) => Error::Network(error_kind),
            Self::Server => Error::Server,
            Self::Alloc => Error::Alloc,
            Self::AuthPacketReceived => Error::AuthPacketReceived,
            Self::Disconnect {
                reason,
                reason_string,
                user_properties,
                server_reference,
            } => Error::Disconnect {
                reason,
                reason_string,
                user_properties: user_properties.into_iter().collect(),
                server_reference,
            },
            Self::RecoveryRequired => Error::RecoveryRequired,
            Self::PacketIdentifierNotInFlight => Error::PacketIdentifierNotInFlight,
            Self::RepublishQoSNotMatching => Error::RepublishQoSNotMatching,
            Self::PacketIdentifierAwaitingPubcomp => Error::PacketIdentifierAwaitingPubcomp,
            Self::PacketMaximumLengthExceeded => Error::PacketMaximumLengthExceeded,
            Self::ServerMaximumPacketSizeExceeded => Error::ServerMaximumPacketSizeExceeded,
            Self::InvalidTopicAlias => Error::InvalidTopicAlias,
            Self::SessionBuffer => Error::SessionBuffer,
            Self::SendQuotaExceeded => Error::SendQuotaExceeded,
            Self::UnsupportedByServer => Error::UnsupportedByServer,
            Self::IllegalDisconnectSessionExpiryInterval => {
                Error::IllegalDisconnectSessionExpiryInterval
            }
        }
    }
}

impl<const MAX_USER_PROPERTIES: usize> From<Reserved> for Error<'_, MAX_USER_PROPERTIES> {
    fn from(_: Reserved) -> Self {
        Self::Server
    }
}

impl<B, const MAX_USER_PROPERTIES: usize> From<RawError<B>> for Error<'_, MAX_USER_PROPERTIES> {
    fn from(e: RawError<B>) -> Self {
        match e {
            RawError::Disconnected => Self::RecoveryRequired,
            RawError::Network(e) => Self::Network(e),
            RawError::Alloc(_) => Self::Alloc,
            RawError::Server => Self::Server,
        }
    }
}

impl<const MAX_USER_PROPERTIES: usize> From<TooLargeToEncode> for Error<'_, MAX_USER_PROPERTIES> {
    fn from(_: TooLargeToEncode) -> Self {
        Self::PacketMaximumLengthExceeded
    }
}
