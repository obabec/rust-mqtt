use core::matches;

use heapless::Vec;

use crate::{
    client::raw::RawError,
    eio::ErrorKind,
    header::Reserved,
    types::{MqttString, MqttStringPair, ReasonCode, TooLargeToEncode},
};

/// The main error returned by [`Client`].
///
/// Distincts between unrecoverable and recoverable errors.
/// Recoverability in this context refers to whether the current network connection can
/// be used for further communication after the error has occured.
///
/// # Recovery
/// - For unrecoverable errors, [`Client::abort`] can be called to send an optional DISCONNECT
///   packet if allowed by specification. You can then try to recover the session by calling
///   [`Client::connect`] again without clean start.
/// - For recoverable errors, follow the error-specific behaviour.
///
/// [`Client`]: crate::client::Client
/// [`Client::abort`]: crate::client::Client::abort
/// [`Client::connect`]: crate::client::Client::connect
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<'e, const MAX_USER_PROPERTIES: usize> {
    /// An underlying Read/Write method returned an error.
    ///
    /// Unrecoverable error. [`Client::abort`] should be called.
    ///
    /// [`Client::abort`]: crate::client::Client::abort
    Network(ErrorKind),

    /// The remote server did something the client does not understand / does not match the specification.
    ///
    /// Unrecoverable error. [`Client::abort`] should be called.
    ///
    /// [`Client::abort`]: crate::client::Client::abort
    Server,

    /// A buffer provision by the [`BufferProvider`] failed. Therefore a packet could not be received
    /// correctly.
    ///
    /// Unrecoverable error. [`Client::abort`] should be called.
    ///
    /// [`BufferProvider`]: crate::buffer::BufferProvider
    /// [`Client::abort`]: crate::client::Client::abort
    Alloc,

    /// An AUTH packet header has been received by the client. AUTH packets are not supported by the client.
    /// The client has scheduled a DISCONNECT packet with [`ReasonCode::ImplementationSpecificError`].
    /// The packet body has not been decoded.
    ///
    /// Unrecoverable error. [`Client::abort`] should be called.
    ///
    /// [`Client::abort`]: crate::client::Client::abort
    AuthPacketReceived,

    /// The client could not connect to the broker or the broker has sent a DISCONNECT packet.
    ///
    /// Unrecoverable error. [`Client::abort`] should be called.
    ///
    /// [`Client::abort`]: crate::client::Client::abort
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
    /// Unrecoverable error. [`Client::abort`] should be called.
    ///
    /// [`Client::abort`]: crate::client::Client::abort
    RecoveryRequired,

    /// A republish or an acknowledgement has been attempted for a packet identifier without an
    /// in flight entry in the session state.
    ///
    /// Recoverable error. No action has been taken by the client.
    PacketIdentifierNotInFlight,

    /// The requested operation requires a free, unused packet identifier which is not available
    /// at the time of the request. The operation may be retried after an indication of a freed
    /// packet identifier such as:
    /// - A completed or aborted outgoing [`QoS::AtLeastOnce`] or [`QoS::ExactlyOnce`]
    ///   publication flow
    /// - A completed or due to disconnection aborted SUBACK / UNSUBACK handshake.
    ///
    /// Recoverable error. No action has been taken by the client.
    ///
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    AllPacketIdentifiersUsed,

    /// [`AckMode::Manual`] is not applicable for outgoing [`QoS::AtMostOnce`] or [`QoS::AtLeastOnce`]
    /// because there are no acknowledgements sent by the client. The same operation will not fail with
    /// this error if requested with [`AckMode::Automatic`].
    ///
    /// Recoverable error. No action has been taken by the client.
    ///
    /// [`AckMode::Manual`]: crate::client::options::AckMode::Manual
    /// [`AckMode::Automatic`]: crate::client::options::AckMode::Automatic
    /// [`QoS::AtMostOnce`]: crate::types::QoS::AtMostOnce
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    ManualAckNotAllowed,

    /// The requested operation of a publication flow is not allowed for this packet identifier because
    /// it uses a different quality of service. This applies in the following cases:
    /// - A republish of a packet with a quality of service that does not match the quality of service
    ///   of the original publication was attempted.
    /// - A manual PUBACK, PUBREC, PUBREL or PUBCOMP was attempted that does not match the quality of
    ///   service of the original publication.
    ///
    /// Recoverable error. No action has been taken by the client.
    QoSMismatched,

    /// The requested operation of a publication flow is not allowed at this stage of its quality of
    /// service specific handshake and would result in a protocol violation if carried out. For the exact
    /// rules of manual acknowledgements, refer to TODO. An exemplary list of cases (potentially missing
    /// some) when this applies is as follows:
    /// - Automatic acknowledgements:
    ///   - A republish of a packet whose corresponding PUBREL packet has already been sent was
    ///     attempted.
    ///   - A republish of a packet is attempted despite there being no disconnection/reconnection
    ///     between the last transmission of the PUBLISH packet.
    /// - Manual acknowledgements:
    ///   - The cases that are true for automatic acknowledgements.
    ///   - A manual PUBACK was attempted after a reconnection, before having received the retransmitted
    ///     PUBLISH packet.
    ///   - A manual PUBREC was attempted after a reconnection, before having received the retransmitted
    ///     PUBLISH packet.
    ///   - A manual PUBREC was attempted despite the PUBREC having been sent before (whether that was
    ///     in automatic or manual mode doesn't matter here) and the is client waiting for the PUBREL.
    ///   - A manual PUBREC was attempted despite the client already having received a PUBREL and having
    ///     to send a PUBCOMP as the next step in the handshake.
    ///   - A manual PUBREL was attempted despite not having received a PUBREC yet.
    ///   - A manual PUBREL was attempted despite the PUBREL having been sent before in the same network
    ///     connection.
    ///   - A manual PUBCOMP was attempted after a reconnection, before having received the
    ///     (re-)transmitted PUBREL packet.
    ///   - A manual PUBCOMP was attempted despite not having received a PUBREL yet.
    ///
    /// Recoverable error. No action has been taken by the client.
    HandshakeStateMismatched,

    /// A reason code not allowed for the requested operation was supplied. Refer to the
    /// documentation of the attempted operation about which reason codes are allowed.
    ///
    /// Recoverable error. No action has been taken by the client.
    IllegalReasonCode,

    /// A packet was too long to encode its length with the variable byte integer.
    ///
    /// This can currently only be returned from [`Client::publish`] or [`Client::republish`].
    ///
    /// Recoverable error. No action has been taken by the client.
    ///
    /// [`Client::publish`]: crate::client::Client::publish
    /// [`Client::republish`]: crate::client::Client::republish
    PacketMaximumLengthExceeded,

    /// A packet is too long and would exceed the servers maximum packet size.
    ///
    /// Recoverable error. No action has been taken by the client.
    ServerMaximumPacketSizeExceeded,

    /// An action was rejected because an internal buffer used for tracking session state is full.
    ///
    /// Recoverable error. Try again after a [`Event`] has been emitted that indicates that buffer
    /// might be free again.
    ///
    /// Example:
    ///     [`Client::subscribe`] returns this error. Wait until a [`Event::Suback`] is received.
    ///     This clears a spot in the subscribe packet identifiers.
    ///
    /// [`Event`]: crate::client::event::Event
    /// [`Client::subscribe`]: crate::client::Client::subscribe
    /// [`Event::Suback`]: crate::client::event::Event::Suback
    SessionBuffer,

    /// A publish now would exceed the server's receive maximum and ultimately cause a protocol error.
    ///
    /// Recoverable error. Try again after either [`Event::PublishAcknowledged`] or
    /// [`Event::PublishComplete`] has been emitted that indicates that buffer might be free again.
    ///
    /// [`Event::PublishAcknowledged`]: crate::client::event::Event::PublishAcknowledged
    /// [`Event::PublishComplete`]: crate::client::event::Event::PublishComplete
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
    /// - a publication with retain set to true being attempted despite retain not being available on the
    ///   server
    /// - a topic alias in an outgoing publication being greater than the server's maximum topic alias value
    ///
    /// Recoverable error. No action has been taken by the client.
    UnsupportedByServer,

    /// A shared subscription with the no local flag set to [`true`] was attempted. This would cause a protocol
    /// error.
    ///
    /// Recoverable error. No action has been taken by the client. Ensure that at max only one of
    /// [`TopicFilter::is_shared`] and the `no_local` field in the [`SubscriptionOptions`] is [`true`].
    ///
    /// [`SubscriptionOptions`]: crate::client::options::SubscriptionOptions
    /// [`TopicFilter::is_shared`]: crate::types::TopicFilter::is_shared
    IllegalNoLocalSharedSubscription,

    /// A disconnect now with the given session expiry interval would cause a protocol error.
    ///
    /// A disconnection was attempted with a session expiry interval change where the session expiry interval in the
    /// CONNECT packet was zero ([`SessionExpiryInterval::EndOnDisconnect`]) and was greater than zero
    /// ([`SessionExpiryInterval::NeverEnd`] or [`SessionExpiryInterval::Seconds`]) in the DISCONNECT packet.
    ///
    /// Recoverable error. Try disconnecting again without an session expiry interval or with a
    /// session expiry interval of zero ([`SessionExpiryInterval::EndOnDisconnect`]).
    ///
    /// [`SessionExpiryInterval::EndOnDisconnect`]: crate::config::SessionExpiryInterval::EndOnDisconnect
    /// [`SessionExpiryInterval::NeverEnd`]: crate::config::SessionExpiryInterval::NeverEnd
    /// [`SessionExpiryInterval::Seconds`]: crate::config::SessionExpiryInterval::Seconds
    IllegalDisconnectSessionExpiryInterval,
}

impl<const MAX_USER_PROPERTIES: usize> Error<'_, MAX_USER_PROPERTIES> {
    /// Returns whether the client can recover from this error without closing the network connection.
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::PacketIdentifierNotInFlight
                | Self::AllPacketIdentifiersUsed
                | Self::ManualAckNotAllowed
                | Self::QoSMismatched
                | Self::HandshakeStateMismatched
                | Self::IllegalReasonCode
                | Self::PacketMaximumLengthExceeded
                | Self::ServerMaximumPacketSizeExceeded
                | Self::SessionBuffer
                | Self::SendQuotaExceeded
                | Self::UnsupportedByServer
                | Self::IllegalNoLocalSharedSubscription
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
            Self::AllPacketIdentifiersUsed => Error::AllPacketIdentifiersUsed,
            Self::ManualAckNotAllowed => Error::ManualAckNotAllowed,
            Self::QoSMismatched => Error::QoSMismatched,
            Self::HandshakeStateMismatched => Error::HandshakeStateMismatched,
            Self::IllegalReasonCode => Error::IllegalReasonCode,
            Self::PacketMaximumLengthExceeded => Error::PacketMaximumLengthExceeded,
            Self::ServerMaximumPacketSizeExceeded => Error::ServerMaximumPacketSizeExceeded,
            Self::SessionBuffer => Error::SessionBuffer,
            Self::SendQuotaExceeded => Error::SendQuotaExceeded,
            Self::UnsupportedByServer => Error::UnsupportedByServer,
            Self::IllegalNoLocalSharedSubscription => Error::IllegalNoLocalSharedSubscription,
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
