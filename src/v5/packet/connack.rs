use crate::{
    buffer::BufferProvider,
    config::{MaximumPacketSize, ReceiveMaximum, SessionExpiryInterval},
    eio::Read,
    fmt::{error, trace},
    header::{FixedHeader, PacketType},
    io::read::{BodyReader, Readable},
    packet::{Packet, RxError, RxPacket},
    types::{ReasonCode, VarByteInt},
    v5::property::{
        AssignedClientIdentifier, AtMostOnceProperty, AuthenticationData, AuthenticationMethod,
        MaximumQoS, PropertyType, ReasonString, ResponseInformation, RetainAvailable,
        ServerKeepAlive, ServerReference, SharedSubscriptionAvailable,
        SubscriptionIdentifierAvailable, TopicAliasMaximum, WildcardSubscriptionAvailable,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConnackPacket<'p> {
    pub session_present: bool,
    pub reason_code: ReasonCode,

    // CONNACK properties
    pub session_expiry_interval: Option<SessionExpiryInterval>,
    pub receive_maximum: Option<ReceiveMaximum>,
    pub maximum_qos: Option<MaximumQoS>,
    pub retain_available: Option<RetainAvailable>,
    pub maximum_packet_size: Option<MaximumPacketSize>,
    pub assigned_client_identifier: Option<AssignedClientIdentifier<'p>>,
    pub topic_alias_maximum: Option<TopicAliasMaximum>,
    pub reason_string: Option<ReasonString<'p>>,
    pub wildcard_subscription_available: Option<WildcardSubscriptionAvailable>,
    pub subscription_identifier_available: Option<SubscriptionIdentifierAvailable>,
    pub shared_subscription_available: Option<SharedSubscriptionAvailable>,
    pub server_keep_alive: Option<ServerKeepAlive>,
    pub response_information: Option<ResponseInformation<'p>>,
    pub server_reference: Option<ServerReference<'p>>,
    pub authentication_method: Option<AuthenticationMethod<'p>>,
    pub authentication_data: Option<AuthenticationData<'p>>,
}

impl<'p> Packet for ConnackPacket<'p> {
    const PACKET_TYPE: PacketType = PacketType::Connack;
}
impl<'p> RxPacket<'p> for ConnackPacket<'p> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding");

        if header.flags() != 0 {
            error!("flags are not 0");
            return Err(RxError::MalformedPacket);
        }
        let r = &mut reader;

        trace!("reading connack flags");
        let connack_flags = u8::read(r).await?;

        trace!("reading connect reason code");
        let connect_reason_code = ReasonCode::read(r).await?;
        if !matches!(
            connect_reason_code,
            ReasonCode::Success
                | ReasonCode::UnspecifiedError
                | ReasonCode::MalformedPacket
                | ReasonCode::ProtocolError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::UnsupportedProtocolVersion
                | ReasonCode::ClientIdentifierNotValid
                | ReasonCode::BadUserNameOrPassword
                | ReasonCode::NotAuthorized
                | ReasonCode::ServerUnavailable
                | ReasonCode::ServerBusy
                | ReasonCode::Banned
                | ReasonCode::BadAuthenticationMethod
                | ReasonCode::TopicNameInvalid
                | ReasonCode::PacketTooLarge
                | ReasonCode::QuotaExceeded
                | ReasonCode::PayloadFormatInvalid
                | ReasonCode::RetainNotSupported
                | ReasonCode::QoSNotSupported
                | ReasonCode::UseAnotherServer
                | ReasonCode::ServerMoved
                | ReasonCode::ConnectionRateExceeded
        ) {
            error!("invalid reason code: {:?}", connect_reason_code);
            return Err(RxError::ProtocolError);
        }

        // first 7 bits have to be set to 0
        if connack_flags & 0xFE > 0 {
            error!("invalid connack flags");
            return Err(RxError::ProtocolError);
        }

        let mut packet = Self {
            session_present: connack_flags > 0,
            reason_code: connect_reason_code,

            session_expiry_interval: None,
            receive_maximum: None,
            maximum_qos: None,
            retain_available: None,
            maximum_packet_size: None,
            assigned_client_identifier: None,
            topic_alias_maximum: None,
            reason_string: None,
            wildcard_subscription_available: None,
            subscription_identifier_available: None,
            shared_subscription_available: None,
            server_keep_alive: None,
            response_information: None,
            server_reference: None,
            authentication_method: None,
            authentication_data: None,
        };

        trace!("reading properties length");
        let properties_length = VarByteInt::read(r).await?.size();

        trace!("properties length = {}", properties_length);

        if r.remaining_len() != properties_length {
            error!("properties length is not equal to remaining packet length");
            return Err(RxError::MalformedPacket);
        }

        while r.remaining_len() > 0 {
            trace!(
                "reading property type with remaining len = {}",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;

            trace!(
                "reading property body of {:?} with remaining len = {}",
                property_type,
                r.remaining_len()
            );
            #[rustfmt::skip]
            match property_type {
                PropertyType::SessionExpiryInterval => packet.session_expiry_interval.try_set(r).await?,
                PropertyType::ReceiveMaximum => packet.receive_maximum.try_set(r).await?,
                PropertyType::MaximumQoS => packet.maximum_qos.try_set(r).await?,
                PropertyType::RetainAvailable => packet.retain_available.try_set(r).await?,
                PropertyType::MaximumPacketSize => packet.maximum_packet_size.try_set(r).await?,
                PropertyType::AssignedClientIdentifier => packet.assigned_client_identifier.try_set(r).await?,
                PropertyType::TopicAliasMaximum => packet.topic_alias_maximum.try_set(r).await?,
                PropertyType::ReasonString => packet.reason_string.try_set(r).await?,
                PropertyType::WildcardSubscriptionAvailable => packet.wildcard_subscription_available.try_set(r).await?,
                PropertyType::SubscriptionIdentifierAvailable => packet.subscription_identifier_available.try_set(r).await?,
                PropertyType::SharedSubscriptionAvailable => packet.shared_subscription_available.try_set(r).await?,
                PropertyType::ServerKeepAlive => packet.server_keep_alive.try_set(r).await?,
                PropertyType::ResponseInformation => packet.response_information.try_set(r).await?,
                PropertyType::ServerReference => packet.server_reference.try_set(r).await?,
                PropertyType::AuthenticationMethod => packet.authentication_method.try_set(r).await?,
                PropertyType::AuthenticationData => packet.authentication_data.try_set(r).await?,
                PropertyType::UserProperty => {
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                },
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    error!("packet contains unexpected property {:?}", p);
                    return Err(RxError::MalformedPacket)
                },
            };
        }

        Ok(packet)
    }
}

#[cfg(test)]
mod unit {
    use crate::{
        config::{KeepAlive, MaximumPacketSize, ReceiveMaximum, SessionExpiryInterval},
        test::rx::decode,
        types::{MqttBinary, MqttString, QoS, ReasonCode},
        v5::{
            packet::ConnackPacket,
            property::{
                AssignedClientIdentifier, AuthenticationData, AuthenticationMethod, MaximumQoS,
                ReasonString, ResponseInformation, RetainAvailable, ServerKeepAlive,
                ServerReference, SharedSubscriptionAvailable, SubscriptionIdentifierAvailable,
                TopicAliasMaximum, WildcardSubscriptionAvailable,
            },
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn decode_simple() {
        let packet = decode!(ConnackPacket, 3, [0x20, 0x03, 0x01, 0x9D, 0x00]);

        assert_eq!(packet.reason_code, ReasonCode::ServerMoved);
        assert!(packet.session_present);

        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.receive_maximum.is_none());
        assert!(packet.maximum_qos.is_none());
        assert!(packet.retain_available.is_none());
        assert!(packet.maximum_packet_size.is_none());
        assert!(packet.assigned_client_identifier.is_none());
        assert!(packet.topic_alias_maximum.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.wildcard_subscription_available.is_none());
        assert!(packet.subscription_identifier_available.is_none());
        assert!(packet.shared_subscription_available.is_none());
        assert!(packet.server_keep_alive.is_none());
        assert!(packet.response_information.is_none());
        assert!(packet.server_reference.is_none());
        assert!(packet.authentication_method.is_none());
        assert!(packet.authentication_data.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(
            ConnackPacket,
            127,
            [
                0x20, // packet type
                0x7F, // remaining length

                0x00, // connect acknowledge flags
                0x00, // reason code
                0x7C, // property length (88 bytes)

                // Session Expiry Interval
                0x11, 0x54, 0x16, 0x68, 0x21,

                // Receive Maximum
                0x21, 0x12, 0x99,

                // Maximum QoS
                0x24, 0x01,

                // Retain Available
                0x25, 0x01,

                // Maximum Packet Size
                0x27, 0x00, 0x10, 0x00, 0x00,

                // Assigned Client Identifier
                0x12, 0x00, 0x09, b'c', b'l', b'i', b'e', b'n', b't', b'1', b'2', b'3',

                // Topic Alias Maximum - 2 bytes
                0x22, 0x00, 0x0A,

                // Reason String
                0x1F, 0x00, 0x02, b'O', b'K',

                // User Property
                0x26, 0x00, 0x03, b'k', b'e', b'y', 0x00, 0x05, b'v', b'a', b'l', b'u', b'e',

                // Wildcard Subscription Available
                0x28, 0x01,

                // Subscription Identifiers Available
                0x29, 0x01,

                // Shared Subscription Available
                0x2A, 0x01,

                // Server Keep Alive
                0x13, 0x00, 0x3C,

                // Response Information
                0x1A, 0x00, 0x0D, b'r', b'e', b's', b'p', b'o', b'n', b's', b'e', b'_', b'i', b'n', b'f', b'o',

                // Server Reference
                0x1C, 0x00, 0x12, b's', b'e', b'r', b'v', b'e', b'r', b'.', b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm',

                // Authentication Method
                0x15, 0x00, 0x0D, b'S', b'C', b'R', b'A', b'M', b'-', b'S', b'H', b'A', b'-', b'2', b'5', b'6',

                // Authentication Data
                0x16, 0x00, 0x09, b'a', b'u', b't', b'h', b'_', b'd', b'a', b't', b'a',
            ]
        );

        assert_eq!(packet.reason_code, ReasonCode::Success);
        assert!(!packet.session_present);

        assert_eq!(
            packet.session_expiry_interval,
            Some(SessionExpiryInterval::Seconds(1410754593))
        );
        assert_eq!(packet.receive_maximum, Some(ReceiveMaximum(4761)));
        assert_eq!(packet.maximum_qos, Some(MaximumQoS(QoS::AtLeastOnce)));
        assert_eq!(packet.retain_available, Some(RetainAvailable(true)));
        assert_eq!(
            packet.maximum_packet_size,
            Some(MaximumPacketSize::Limit(1048576))
        );
        assert_eq!(
            packet.assigned_client_identifier,
            Some(AssignedClientIdentifier(
                MqttString::try_from("client123").unwrap()
            ))
        );
        assert_eq!(packet.topic_alias_maximum, Some(TopicAliasMaximum(10)));
        assert_eq!(
            packet.reason_string,
            Some(ReasonString(MqttString::try_from("OK").unwrap()))
        );
        assert_eq!(
            packet.wildcard_subscription_available,
            Some(WildcardSubscriptionAvailable(true))
        );
        assert_eq!(
            packet.subscription_identifier_available,
            Some(SubscriptionIdentifierAvailable(true))
        );
        assert_eq!(
            packet.shared_subscription_available,
            Some(SharedSubscriptionAvailable(true))
        );
        assert_eq!(
            packet.server_keep_alive,
            Some(ServerKeepAlive(KeepAlive::Seconds(60)))
        );
        assert_eq!(
            packet.response_information,
            Some(ResponseInformation(
                MqttString::try_from("response_info").unwrap()
            ))
        );
        assert_eq!(
            packet.server_reference,
            Some(ServerReference(
                MqttString::try_from("server.example.com").unwrap()
            ))
        );
        assert_eq!(
            packet.authentication_method,
            Some(AuthenticationMethod(
                MqttString::try_from("SCRAM-SHA-256").unwrap()
            ))
        );
        assert_eq!(
            packet.authentication_data,
            Some(AuthenticationData(
                MqttBinary::try_from("auth_data".as_bytes()).unwrap()
            ))
        );
    }
}
