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
    /// # aka `GrantedQoS0`, `NormalDisconnection`
    ///
    /// CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBACK, UNSUBACK, AUTH, DISCONNECT
    Success = 0x00,

    /// SUBACK
    GrantedQoS1 = 0x01,

    /// SUBACK
    GrantedQoS2 = 0x02,

    /// DISCONNECT
    DisconnectWithWillMessage = 0x04,

    /// PUBACK, PUBREC
    NoMatchingSubscribers = 0x10,

    /// UNSUBACK
    NoSubscriptionExisted = 0x11,

    /// AUTH
    ContinueAuthentication = 0x18,

    /// AUTH
    ReAuthenticate = 0x19,

    #[default]
    /// CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    UnspecifiedError = 0x80,

    /// CONNACK, DISCONNECT
    MalformedPacket = 0x81,

    /// CONNACK, DISCONNECT
    ProtocolError = 0x82,

    /// CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    ImplementationSpecificError = 0x83,

    /// CONNACK
    UnsupportedProtocolVersion = 0x84,

    /// CONNACK
    ClientIdentifierNotValid = 0x85,

    /// CONNACK
    BadUserNameOrPassword = 0x86,

    /// CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    NotAuthorized = 0x87,

    /// CONNACK
    ServerUnavailable = 0x88,

    /// CONNACK, DISCONNECT
    ServerBusy = 0x89,

    /// CONNACK
    Banned = 0x8A,

    /// DISCONNECT
    ServerShuttingDown = 0x8B,

    /// CONNACK, DISCONNECT
    BadAuthenticationMethod = 0x8C,

    /// DISCONNECT
    KeepAliveTimeout = 0x8D,

    /// DISCONNECT
    SessionTakenOver = 0x8E,

    /// SUBACK, UNSUBACK, DISCONNECT
    TopicFilterInvalid = 0x8F,

    /// CONNACK, PUBACK, PUBREC, DISCONNECT
    TopicNameInvalid = 0x90,

    /// PUBACK, PUBREC, SUBACK, UNSUBACK
    PacketIdentifierInUse = 0x91,

    /// PUBREL, PUBCOMP
    PacketIdentifierNotFound = 0x92,

    /// DISCONNECT
    ReceiveMaximumExceeded = 0x93,

    /// DISCONNECT
    TopicAliasInvalid = 0x94,

    /// CONNACK, DISCONNECT
    PacketTooLarge = 0x95,

    /// DISCONNECT
    MessageRateTooHigh = 0x96,

    /// CONNACK, PUBACK, PUBREC, SUBACK, DISCONNECT
    QuotaExceeded = 0x97,

    /// DISCONNECT
    AdministrativeAction = 0x98,

    /// CONNACK, PUBACK, PUBREC, DISCONNECT
    PayloadFormatInvalid = 0x99,

    /// CONNACK, DISCONNECT
    RetainNotSupported = 0x9A,

    /// CONNACK, DISCONNECT
    QoSNotSupported = 0x9B,

    /// CONNACK, DISCONNECT
    UseAnotherServer = 0x9C,

    /// CONNACK, DISCONNECT
    ServerMoved = 0x9D,

    /// SUBACK, DISCONNECT
    SharedSubscriptionsNotSupported = 0x9E,

    /// CONNACK, DISCONNECT
    ConnectionRateExceeded = 0x9F,

    /// DISCONNECT
    MaximumConnectTime = 0xA0,

    /// SUBACK, DISCONNECT
    SubscriptionIdentifiersNotSupported = 0xA1,

    /// SUBACK, DISCONNECT
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
