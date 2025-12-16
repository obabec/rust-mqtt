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
pub enum ReasonCode {
    /// # aka GrantedQoS0, NormalDisconnection
    ///
    /// CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBACK, UNSUBACK, AUTH, DISCONNECT
    Success,

    /// SUBACK
    GrantedQoS1,

    /// SUBACK
    GrantedQoS2,

    /// DISCONNECT
    DisconnectWithWillMessage,

    /// PUBACK, PUBREC
    NoMatchingSubscribers,

    /// UNSUBACK
    NoSubscriptionExisted,

    /// AUTH
    ContinueAuthentication,

    /// AUTH
    ReAuthenticate,

    #[default]
    /// CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    UnspecifiedError,

    /// CONNACK, DISCONNECT
    MalformedPacket,

    /// CONNACK, DISCONNECT
    ProtocolError,

    /// CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    ImplementationSpecificError,

    /// CONNACK
    UnsupportedProtocolVersion,

    /// CONNACK
    ClientIdentifierNotValid,

    /// CONNACK
    BadUserNameOrPassword,

    /// CONNACK, PUBACK, PUBREC, SUBACK, UNSUBACK, DISCONNECT
    NotAuthorized,

    /// CONNACK
    ServerUnavailable,

    /// CONNACK, DISCONNECT
    ServerBusy,

    /// CONNACK
    Banned,

    /// DISCONNECT
    ServerShuttingDown,

    /// CONNACK, DISCONNECT
    BadAuthenticationMethod,

    /// DISCONNECT
    KeepAliveTimeout,

    /// DISCONNECT
    SessionTakenOver,

    /// SUBACK, UNSUBACK, DISCONNECT
    TopicFilterInvalid,

    /// CONNACK, PUBACK, PUBREC, DISCONNECT
    TopicNameInvalid,

    /// PUBACK, PUBREC, SUBACK, UNSUBACK
    PacketIdentifierInUse,

    /// PUBREL, PUBCOMP
    PacketIdentifierNotFound,

    /// DISCONNECT
    ReceiveMaximumExceeded,

    /// DISCONNECT
    TopicAliasInvalid,

    /// CONNACK, DISCONNECT
    PacketTooLarge,

    /// DISCONNECT
    MessageRateTooHigh,

    /// CONNACK, PUBACK, PUBREC, SUBACK, DISCONNECT
    QuotaExceeded,

    /// DISCONNECT
    AdministrativeAction,

    /// CONNACK, PUBACK, PUBREC, DISCONNECT
    PayloadFormatInvalid,

    /// CONNACK, DISCONNECT
    RetainNotSupported,

    /// CONNACK, DISCONNECT
    QoSNotSupported,

    /// CONNACK, DISCONNECT
    UseAnotherServer,

    /// CONNACK, DISCONNECT
    ServerMoved,

    /// SUBACK, DISCONNECT
    SharedSubscriptionsNotSupported,

    /// CONNACK, DISCONNECT
    ConnectionRateExceeded,

    /// DISCONNECT
    MaximumConnectTime,

    /// SUBACK, DISCONNECT
    SubscriptionIdentifiersNotSupported,

    /// SUBACK, DISCONNECT
    WildcardSubscriptionsNotSupported,
}

impl ReasonCode {
    /// Returns the numeric value of the reason code.
    pub fn value(&self) -> u8 {
        match self {
            Self::Success => 0x00,
            Self::GrantedQoS1 => 0x01,
            Self::GrantedQoS2 => 0x02,
            Self::DisconnectWithWillMessage => 0x04,
            Self::NoMatchingSubscribers => 0x10,
            Self::NoSubscriptionExisted => 0x11,
            Self::ContinueAuthentication => 0x18,
            Self::ReAuthenticate => 0x19,
            Self::UnspecifiedError => 0x80,
            Self::MalformedPacket => 0x81,
            Self::ProtocolError => 0x82,
            Self::ImplementationSpecificError => 0x83,
            Self::UnsupportedProtocolVersion => 0x84,
            Self::ClientIdentifierNotValid => 0x85,
            Self::BadUserNameOrPassword => 0x86,
            Self::NotAuthorized => 0x87,
            Self::ServerUnavailable => 0x88,
            Self::ServerBusy => 0x89,
            Self::Banned => 0x8A,
            Self::ServerShuttingDown => 0x8B,
            Self::BadAuthenticationMethod => 0x8C,
            Self::KeepAliveTimeout => 0x8D,
            Self::SessionTakenOver => 0x8E,
            Self::TopicFilterInvalid => 0x8F,
            Self::TopicNameInvalid => 0x90,
            Self::PacketIdentifierInUse => 0x91,
            Self::PacketIdentifierNotFound => 0x92,
            Self::ReceiveMaximumExceeded => 0x93,
            Self::TopicAliasInvalid => 0x94,
            Self::PacketTooLarge => 0x95,
            Self::MessageRateTooHigh => 0x96,
            Self::QuotaExceeded => 0x97,
            Self::AdministrativeAction => 0x98,
            Self::PayloadFormatInvalid => 0x99,
            Self::RetainNotSupported => 0x9A,
            Self::QoSNotSupported => 0x9B,
            Self::UseAnotherServer => 0x9C,
            Self::ServerMoved => 0x9D,
            Self::SharedSubscriptionsNotSupported => 0x9E,
            Self::ConnectionRateExceeded => 0x9F,
            Self::MaximumConnectTime => 0xA0,
            Self::SubscriptionIdentifiersNotSupported => 0xA1,
            Self::WildcardSubscriptionsNotSupported => 0xA2,
        }
    }

    /// Returns whether the reason code is successful.
    /// This is the case if the reason code's numeric value is less than 0x80.
    pub fn is_success(&self) -> bool {
        self.value() < 0x80
    }

    /// Returns whether the reason code indicates an error.
    /// This is the case if the reason code's numeric value is greater than or equal to 0x80.
    pub fn is_erroneous(&self) -> bool {
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
