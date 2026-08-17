use crate::{
    eio::{Read, Write},
    io::{
        err::{ReadError, WriteError},
        read::Readable,
        write::Writable,
    },
};

/// A Reason Code is a variable byte integer encoded value that indicates the result of an operation.
///
/// The CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, DISCONNECT and AUTH Control Packets have
/// a single Reason Code as part of the Variable Header. The SUBACK and UNSUBACK packets
/// contain a list of one or more Reason Codes in the Payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum ReasonCode {
    /// - CONNACK: The Connection is accepted.
    /// - PUBACK: The message is accepted. Publication of the QoS 1 message proceeds.
    /// - PUBREC: The message is accepted. Publication of the QoS 2 message proceeds.
    /// - PUBREL: Message released.
    /// - PUBCOMP: Packet Identifier released. Publication of QoS 2 message is complete.
    /// - SUBACK (`GrantedQoS0`): The subscription is accepted and the maximum QoS sent will be QoS 0. This might be a lower QoS than was requested.
    /// - UNSUBACK: The subscription is deleted.
    /// - DISCONNECT (`NormalDisconnection`): Close the connection normally. Do not send the Will Message.
    /// - AUTH: Authentication is successful.
    Success = 0x00,

    /// - SUBACK: The subscription is accepted and the maximum QoS sent will be QoS 1. This might be a lower QoS than was requested.
    GrantedQoS1 = 0x01,

    /// - SUBACK: The subscription is accepted and any received QoS will be sent to this subscription.
    GrantedQoS2 = 0x02,

    /// - DISCONNECT (Client only): The Client wishes to disconnect but requires that the Server also publishes its Will Message.
    DisconnectWithWillMessage = 0x04,

    /// - PUBACK: The message is accepted but there are no subscribers. This is sent only by the Server. If the Server knows that there are no matching subscribers, it MAY use this Reason Code instead of 0x00 (Success).
    /// - PUBREC: The message is accepted but there are no subscribers. This is sent only by the Server. If the Server knows that there are no matching subscribers, it MAY use this Reason Code instead of 0x00 (Success).
    NoMatchingSubscribers = 0x10,

    /// - UNSUBACK: No matching Topic Filter is being used by the Client.
    NoSubscriptionExisted = 0x11,

    /// - AUTH: Continue the authentication with another step.
    ContinueAuthentication = 0x18,

    /// - AUTH: Initiate a re-authentication.
    ReAuthenticate = 0x19,

    #[default]
    /// - CONNACK: The Server does not wish to reveal the reason for the failure, or none of the other Reason Codes apply.
    /// - PUBACK: The receiver does not accept the publish but either does not want to reveal the reason, or it does not match one of the other values.
    /// - PUBREC: The receiver does not accept the publish but either does not want to reveal the reason, or it does not match one of the other values.
    /// - SUBACK: The subscription is not accepted and the Server either does not wish to reveal the reason or none of the other Reason Codes apply.
    /// - UNSUBACK: The unsubscribe could not be completed and the Server either does not wish to reveal the reason or none of the other Reason Codes apply.
    /// - DISCONNECT: The Connection is closed but the sender either does not wish to reveal the reason, or none of the other Reason Codes apply.
    UnspecifiedError = 0x80,

    /// - CONNACK: Data within the CONNECT packet could not be correctly parsed.
    /// - DISCONNECT: The received packet does not conform to this specification.
    MalformedPacket = 0x81,

    /// - CONNACK: Data in the CONNECT packet does not conform to this specification.
    /// - DISCONNECT: An unexpected or out of order packet was received.
    ProtocolError = 0x82,

    /// - CONNACK: The CONNECT is valid but is not accepted by this Server.
    /// - PUBACK: The PUBLISH is valid but the receiver is not willing to accept it.
    /// - PUBREC: The PUBLISH is valid but the receiver is not willing to accept it.
    /// - SUBACK: The SUBSCRIBE is valid but the Server does not accept it.
    /// - UNSUBACK: The UNSUBSCRIBE is valid but the Server does not accept it.
    /// - DISCONNECT: The packet received is valid but cannot be processed by this implementation.
    ImplementationSpecificError = 0x83,

    /// - CONNACK: The Server does not support the version of the MQTT protocol requested by the Client.
    UnsupportedProtocolVersion = 0x84,

    /// - CONNACK: The Client Identifier is a valid string but is not allowed by the Server.
    ClientIdentifierNotValid = 0x85,

    /// - CONNACK: The Server does not accept the User Name or Password specified by the Client.
    BadUserNameOrPassword = 0x86,

    /// - CONNACK: The Client is not authorized to connect.
    /// - PUBACK: The PUBLISH is not authorized.
    /// - PUBREC: The PUBLISH is not authorized.
    /// - SUBACK: The Client is not authorized to make this subscription.
    /// - UNSUBACK: The Client is not authorized to unsubscribe.
    /// - DISCONNECT (Server only): The request is not authorized.
    NotAuthorized = 0x87,

    /// - CONNACK: The MQTT Server is not available.
    ServerUnavailable = 0x88,

    /// - CONNACK: The Server is busy. Try again later.
    /// - DISCONNECT (Server only): The Server is busy and cannot continue processing requests from this Client.
    ServerBusy = 0x89,

    /// - CONNACK: This Client has been banned by administrative action. Contact the server administrator.
    Banned = 0x8A,

    /// - DISCONNECT (Server only): The Server is shutting down.
    ServerShuttingDown = 0x8B,

    /// - CONNACK: The authentication method is not supported or does not match the authentication method currently in use.
    BadAuthenticationMethod = 0x8C,

    /// - DISCONNECT (Server only): The Connection is closed because no packet has been received for 1.5 times the Keepalive time.
    KeepAliveTimeout = 0x8D,

    /// - DISCONNECT (Server only): Another Connection using the same ClientID has connected causing this Connection to be closed.
    SessionTakenOver = 0x8E,

    /// - SUBACK: The Topic Filter is correctly formed but is not allowed for this Client.
    /// - UNSUBACK: The Topic Filter is correctly formed but is not allowed for this Client.
    /// - DISCONNECT (Server only): The Topic Filter is correctly formed, but is not accepted by this Sever.
    TopicFilterInvalid = 0x8F,

    /// - CONNACK: The Will Topic Name is not malformed, but is not accepted by this Server.
    /// - PUBACK: The Topic Name is not malformed, but is not accepted by this Client or Server.
    /// - PUBREC: The Topic Name is not malformed, but is not accepted by this Client or Server.
    /// - DISCONNECT: The Topic Name is correctly formed, but is not accepted by this Client or Server.
    TopicNameInvalid = 0x90,

    /// - PUBACK: The Packet Identifier is already in use. This might indicate a mismatch in the Session State between the Client and Server.
    /// - PUBREC: The Packet Identifier is already in use. This might indicate a mismatch in the Session State between the Client and Server.
    /// - SUBACK: The specified Packet Identifier is already in use.
    /// - UNSUBACK: The specified Packet Identifier is already in use.
    PacketIdentifierInUse = 0x91,

    /// - PUBREL: The Packet Identifier is not known. This is not an error during recovery, but at other times indicates a mismatch between the Session State on the Client and Server.
    /// - PUBCOMP: The Packet Identifier is not known. This is not an error during recovery, but at other times indicates a mismatch between the Session State on the Client and Server.
    PacketIdentifierNotFound = 0x92,

    /// - DISCONNECT: The Client or Server has received more than Receive Maximum publication for which it has not sent PUBACK or PUBCOMP.
    ReceiveMaximumExceeded = 0x93,

    /// - DISCONNECT: The Client or Server has received a PUBLISH packet containing a Topic Alias which is greater than the Maximum Topic Alias it sent in the CONNECT or CONNACK packet.
    TopicAliasInvalid = 0x94,

    /// - CONNACK: The CONNECT packet exceeded the maximum permissible size.
    /// - DISCONNECT: The packet size is greater than Maximum Packet Size for this Client or Server.
    PacketTooLarge = 0x95,

    /// - DISCONNECT: The received data rate is too high.
    MessageRateTooHigh = 0x96,

    /// - CONNACK: An implementation or administrative imposed limit has been exceeded.
    /// - PUBACK: An implementation or administrative imposed limit has been exceeded.
    /// - PUBREC: An implementation or administrative imposed limit has been exceeded.
    /// - SUBACK: An implementation or administrative imposed limit has been exceeded.
    /// - DISCONNECT: An implementation or administrative imposed limit has been exceeded.
    QuotaExceeded = 0x97,

    /// - DISCONNECT: The Connection is closed due to an administrative action.
    AdministrativeAction = 0x98,

    /// - CONNACK: The Will Payload does not match the specified Payload Format Indicator.
    /// - PUBACK: The payload format does not match the specified Payload Format Indicator.
    /// - PUBREC: The payload format does not match the one specified in the Payload Format Indicator.
    /// - DISCONNECT: The payload format does not match the one specified by the Payload Format Indicator.
    PayloadFormatInvalid = 0x99,

    /// - CONNACK: The Server does not support retained messages, and Will Retain was set to 1.
    /// - DISCONNECT (Server only): The Server has does not support retained messages.
    RetainNotSupported = 0x9A,

    /// - CONNACK: The Server does not support the QoS set in Will QoS.
    /// - DISCONNECT (Server only): The Client specified a QoS greater than the QoS specified in a Maximum QoS in the CONNACK.
    QoSNotSupported = 0x9B,

    /// - CONNACK: The Client should temporarily use another server.
    /// - DISCONNECT (Server only): The Client should temporarily change its Server.
    UseAnotherServer = 0x9C,

    /// - CONNACK: The Client should permanently use another server.
    /// - DISCONNECT (Server only): The Server is moved and the Client should permanently change its server location.
    ServerMoved = 0x9D,

    /// - SUBACK: The Server does not support Shared Subscriptions for this Client.
    /// - DISCONNECT (Server only): The Server does not support Shared Subscriptions.
    SharedSubscriptionsNotSupported = 0x9E,

    /// - CONNACK: The connection rate limit has been exceeded.
    /// - DISCONNECT (Server only): This connection is closed because the connection rate is too high.
    ConnectionRateExceeded = 0x9F,

    /// - DISCONNECT (Server only): The maximum connection time authorized for this connection has been exceeded.
    MaximumConnectTime = 0xA0,

    /// - SUBACK: The Server does not support Subscription Identifiers; the subscription is not accepted.
    /// - DISCONNECT (Server only): The Server does not support Subscription Identifiers; the subscription is not accepted.
    SubscriptionIdentifiersNotSupported = 0xA1,

    /// - SUBACK: The Server does not support Wildcard Subscriptions; the subscription is not accepted.
    /// - DISCONNECT (Server only): The Server does not support Wildcard Subscriptions; the subscription is not accepted.
    WildcardSubscriptionsNotSupported = 0xA2,
}

impl ReasonCode {
    /// Returns the numeric value of the reason code.
    #[must_use]
    pub const fn value(self) -> u8 {
        self as u8
    }

    /// Returns whether the reason code is successful.
    /// This is the case if the reason code's numeric value is less than 0x80.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.value() < 0x80
    }

    /// Returns whether the reason code indicates an error.
    /// This is the case if the reason code's numeric value is greater than or equal to 0x80.
    #[must_use]
    pub const fn is_erroneous(&self) -> bool {
        self.value() >= 0x80
    }
}

impl<R: Read> Readable<R> for ReasonCode {
    async fn read(net: &mut R) -> Result<Self, ReadError<R::Error>> {
        let value = u8::read(net).await?;
        Ok(match value {
            0x00 => Self::Success, // Note: This is ambiguous - context determines the specific variant
            0x01 => Self::GrantedQoS1,
            0x02 => Self::GrantedQoS2,
            0x04 => Self::DisconnectWithWillMessage,
            0x10 => Self::NoMatchingSubscribers,
            0x11 => Self::NoSubscriptionExisted,
            0x18 => Self::ContinueAuthentication,
            0x19 => Self::ReAuthenticate,
            0x80 => Self::UnspecifiedError,
            0x81 => Self::MalformedPacket,
            0x82 => Self::ProtocolError,
            0x83 => Self::ImplementationSpecificError,
            0x84 => Self::UnsupportedProtocolVersion,
            0x85 => Self::ClientIdentifierNotValid,
            0x86 => Self::BadUserNameOrPassword,
            0x87 => Self::NotAuthorized,
            0x88 => Self::ServerUnavailable,
            0x89 => Self::ServerBusy,
            0x8A => Self::Banned,
            0x8B => Self::ServerShuttingDown,
            0x8C => Self::BadAuthenticationMethod,
            0x8D => Self::KeepAliveTimeout,
            0x8E => Self::SessionTakenOver,
            0x8F => Self::TopicFilterInvalid,
            0x90 => Self::TopicNameInvalid,
            0x91 => Self::PacketIdentifierInUse,
            0x92 => Self::PacketIdentifierNotFound,
            0x93 => Self::ReceiveMaximumExceeded,
            0x94 => Self::TopicAliasInvalid,
            0x95 => Self::PacketTooLarge,
            0x96 => Self::MessageRateTooHigh,
            0x97 => Self::QuotaExceeded,
            0x98 => Self::AdministrativeAction,
            0x99 => Self::PayloadFormatInvalid,
            0x9A => Self::RetainNotSupported,
            0x9B => Self::QoSNotSupported,
            0x9C => Self::UseAnotherServer,
            0x9D => Self::ServerMoved,
            0x9E => Self::SharedSubscriptionsNotSupported,
            0x9F => Self::ConnectionRateExceeded,
            0xA0 => Self::MaximumConnectTime,
            0xA1 => Self::SubscriptionIdentifiersNotSupported,
            0xA2 => Self::WildcardSubscriptionsNotSupported,
            _ => return Err(ReadError::ProtocolError),
        })
    }
}

impl Writable for ReasonCode {
    fn written_len(&self) -> usize {
        1
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.value().write(write).await
    }
}
