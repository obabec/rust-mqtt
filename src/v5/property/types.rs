use crate::{
    eio::{Read, Write},
    io::{
        err::{ReadError, WriteError},
        read::Readable,
        write::Writable,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PropertyType {
    /// PUBLISH, Will Properties
    PayloadFormatIndicator,

    /// PUBLISH, Will Properties
    MessageExpiryInterval,

    /// PUBLISH, Will Properties
    ContentType,

    /// PUBLISH, Will Properties
    ResponseTopic,

    /// PUBLISH, Will Properties
    CorrelationData,

    /// PUBLISH, SUBSCRIBE
    SubscriptionIdentifier,

    /// CONNECT, CONNACK, DISCONNECT
    SessionExpiryInterval,

    /// CONNACK
    AssignedClientIdentifier,

    /// CONNACK
    ServerKeepAlive,

    /// CONNECT, CONNACK, AUTH
    AuthenticationMethod,

    /// CONNET, CONNACK, AUTH
    AuthenticationData,

    /// CONNECT
    RequestProblemInformation,

    /// Will Properties
    WillDelayInterval,

    /// CONNECT
    RequestResponseInformation,

    /// CONNACK
    ResponseInformation,

    /// CONNACK, DISCONNECT
    ServerReference,

    /// CONNACK, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBACK, UNSUBACK, DISCONNECT, AUTH
    ReasonString,

    /// CONNECT, CONNACK
    ReceiveMaximum,

    /// CONNECT, CONNACK
    TopicAliasMaximum,

    /// PUBLISH
    TopicAlias,

    /// CONNACK
    MaximumQoS,

    /// CONNACK
    RetainAvailable,

    /// CONNECT, CONNACK, PUBLISH, Will Properties, PUBACK, PUBREC, PUBREL, PUBCOMP, SUBSCRIBE, SUBACK, UNSUBSCRIBE, UNSUBACK, DISCONNECT, AUTH
    UserProperty,

    /// CONNECT, CONNACK
    MaximumPacketSize,

    /// CONNACK
    WildcardSubscriptionAvailable,

    /// CONNACK
    SubscriptionIdentifierAvailable,

    /// CONNACK
    SharedSubscriptionAvailable,
}

impl PropertyType {
    pub const fn from_identifier(identifier: u8) -> Result<Self, ()> {
        Ok(match identifier {
            0x01 => Self::PayloadFormatIndicator,
            0x02 => Self::MessageExpiryInterval,
            0x03 => Self::ContentType,
            0x08 => Self::ResponseTopic,
            0x09 => Self::CorrelationData,
            0x0B => Self::SubscriptionIdentifier,
            0x11 => Self::SessionExpiryInterval,
            0x12 => Self::AssignedClientIdentifier,
            0x13 => Self::ServerKeepAlive,
            0x15 => Self::AuthenticationMethod,
            0x16 => Self::AuthenticationData,
            0x17 => Self::RequestProblemInformation,
            0x18 => Self::WillDelayInterval,
            0x19 => Self::RequestResponseInformation,
            0x1A => Self::ResponseInformation,
            0x1C => Self::ServerReference,
            0x1F => Self::ReasonString,
            0x21 => Self::ReceiveMaximum,
            0x22 => Self::TopicAliasMaximum,
            0x23 => Self::TopicAlias,
            0x24 => Self::MaximumQoS,
            0x25 => Self::RetainAvailable,
            0x26 => Self::UserProperty,
            0x27 => Self::MaximumPacketSize,
            0x28 => Self::WildcardSubscriptionAvailable,
            0x29 => Self::SubscriptionIdentifierAvailable,
            0x2A => Self::SharedSubscriptionAvailable,
            _ => return Err(()),
        })
    }
    pub const fn identifier(&self) -> u8 {
        match self {
            Self::PayloadFormatIndicator => 0x01,
            Self::MessageExpiryInterval => 0x02,
            Self::ContentType => 0x03,
            Self::ResponseTopic => 0x08,
            Self::CorrelationData => 0x09,
            Self::SubscriptionIdentifier => 0x0B,
            Self::SessionExpiryInterval => 0x11,
            Self::AssignedClientIdentifier => 0x12,
            Self::ServerKeepAlive => 0x13,
            Self::AuthenticationMethod => 0x15,
            Self::AuthenticationData => 0x16,
            Self::RequestProblemInformation => 0x17,
            Self::WillDelayInterval => 0x18,
            Self::RequestResponseInformation => 0x19,
            Self::ResponseInformation => 0x1A,
            Self::ServerReference => 0x1C,
            Self::ReasonString => 0x1F,
            Self::ReceiveMaximum => 0x21,
            Self::TopicAliasMaximum => 0x22,
            Self::TopicAlias => 0x23,
            Self::MaximumQoS => 0x24,
            Self::RetainAvailable => 0x25,
            Self::UserProperty => 0x26,
            Self::MaximumPacketSize => 0x27,
            Self::WildcardSubscriptionAvailable => 0x28,
            Self::SubscriptionIdentifierAvailable => 0x29,
            Self::SharedSubscriptionAvailable => 0x2A,
        }
    }
}

impl<R: Read> Readable<R> for PropertyType {
    async fn read(net: &mut R) -> Result<Self, ReadError<R::Error>> {
        let identifier = u8::read(net).await?;

        Self::from_identifier(identifier).map_err(|_| ReadError::MalformedPacket)
    }
}

impl Writable for PropertyType {
    fn written_len(&self) -> usize {
        1
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let identifier: u8 = self.identifier();
        identifier.write(write).await
    }
}
