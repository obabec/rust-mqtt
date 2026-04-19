use heapless::Vec;

use crate::{
    buffer::BufferProvider,
    config::{MaximumPacketSize, SessionExpiryInterval},
    eio::Read,
    fmt::{trace, verbose},
    header::{FixedHeader, PacketType},
    io::read::{BodyReader, Readable},
    packet::{Packet, RxError, RxPacket},
    types::{ReasonCode, VarByteInt},
    v5::property::{
        AssignedClientIdentifier, AtMostOnceProperty, MaximumQoS, PropertyType, ReasonString,
        ReceiveMaximum, ResponseInformation, RetainAvailable, ServerKeepAlive, ServerReference,
        SharedSubscriptionAvailable, SubscriptionIdentifierAvailable, TopicAliasMaximum,
        UserProperty, WildcardSubscriptionAvailable,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConnackPacket<'p, const MAX_USER_PROPERTIES: usize> {
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
    pub user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
    pub wildcard_subscription_available: Option<WildcardSubscriptionAvailable>,
    pub subscription_identifier_available: Option<SubscriptionIdentifierAvailable>,
    pub shared_subscription_available: Option<SharedSubscriptionAvailable>,
    pub server_keep_alive: Option<ServerKeepAlive>,
    pub response_information: Option<ResponseInformation<'p>>,
    pub server_reference: Option<ServerReference<'p>>,
    // authentication method is currently unused and does not have to be read into memory.
    // pub authentication_method: Option<AuthenticationMethod<'p>>,
    // authentication data is currently unused and does not have to be read into memory.
    // pub authentication_data: Option<AuthenticationData<'p>>,
}

impl<const MAX_USER_PROPERTIES: usize> Packet for ConnackPacket<'_, MAX_USER_PROPERTIES> {
    const PACKET_TYPE: PacketType = PacketType::Connack;
}
impl<'p, const MAX_USER_PROPERTIES: usize> RxPacket<'p> for ConnackPacket<'p, MAX_USER_PROPERTIES> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding CONNACK packet");

        if header.flags() != 0 {
            trace!("invalid CONNACK fixed header flags: {}", header.flags());
            return Err(RxError::MalformedPacket);
        }
        let r = &mut reader;

        verbose!("reading CONNACK flags field");
        let connack_flags = u8::read(r).await?;

        verbose!("reading reason code field");
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
            trace!("invalid CONNACK reason code: {:?}", connect_reason_code);
            return Err(RxError::ProtocolError);
        }

        // first 7 bits have to be set to 0
        if connack_flags & 0xFE > 0 {
            trace!("invalid CONNACK variable header flags: {}", connack_flags);
            return Err(RxError::ProtocolError);
        }

        verbose!("reading property length field");
        let properties_length = VarByteInt::read(r).await?.size();

        verbose!("property length: {} bytes", properties_length);

        if r.remaining_len() != properties_length {
            trace!("invalid CONNACK property length for remaining packet length");
            return Err(RxError::MalformedPacket);
        }

        let mut session_expiry_interval = None;
        let mut receive_maximum = None;
        let mut maximum_qos = None;
        let mut retain_available = None;
        let mut maximum_packet_size = None;
        let mut assigned_client_identifier = None;
        let mut topic_alias_maximum = None;
        let mut reason_string = None;
        let mut user_properties = Vec::new();
        let mut wildcard_subscription_available = None;
        let mut subscription_identifier_available = None;
        let mut shared_subscription_available = None;
        let mut server_keep_alive = None;
        let mut response_information = None;
        let mut server_reference = None;

        let mut seen_authentication_method = false;
        let mut seen_authentication_data = false;

        while r.remaining_len() > 0 {
            verbose!(
                "reading property identifier (remaining length: {} bytes)",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;

            verbose!(
                "reading {:?} property body (remaining length: {} bytes)",
                property_type,
                r.remaining_len()
            );
            #[rustfmt::skip]
            match property_type {
                PropertyType::SessionExpiryInterval => session_expiry_interval.try_set(r).await?,
                PropertyType::ReceiveMaximum => receive_maximum.try_set(r).await?,
                PropertyType::MaximumQoS => maximum_qos.try_set(r).await?,
                PropertyType::RetainAvailable => retain_available.try_set(r).await?,
                PropertyType::MaximumPacketSize => maximum_packet_size.try_set(r).await?,
                PropertyType::AssignedClientIdentifier => assigned_client_identifier.try_set(r).await?,
                PropertyType::TopicAliasMaximum => topic_alias_maximum.try_set(r).await?,
                PropertyType::ReasonString => reason_string.try_set(r).await?,
                PropertyType::UserProperty if !user_properties.is_full() => {
                    let user_property = UserProperty::read(r).await?;

                    // Safety: `!Vec::is_full` guarantees there is space
                    unsafe { user_properties.push_unchecked(user_property) };
                }
                PropertyType::UserProperty => {
                    UserProperty::skip(r).await?;
                }
                PropertyType::WildcardSubscriptionAvailable => wildcard_subscription_available.try_set(r).await?,
                PropertyType::SubscriptionIdentifierAvailable => subscription_identifier_available.try_set(r).await?,
                PropertyType::SharedSubscriptionAvailable => shared_subscription_available.try_set(r).await?,
                PropertyType::ServerKeepAlive => server_keep_alive.try_set(r).await?,
                PropertyType::ResponseInformation => response_information.try_set(r).await?,
                PropertyType::ServerReference => server_reference.try_set(r).await?,
                PropertyType::AuthenticationMethod if seen_authentication_method => return Err(RxError::ProtocolError),
                PropertyType::AuthenticationMethod => {
                    seen_authentication_method = true;
                    let len = u16::read(r).await? as usize;
                    verbose!("skipping authentication method ({} bytes)", len);
                    r.skip(len).await?;
                }
                PropertyType::AuthenticationData if seen_authentication_data => return Err(RxError::ProtocolError),
                PropertyType::AuthenticationData => {
                    seen_authentication_data = true;
                    let len = u16::read(r).await? as usize;
                    verbose!("skipping authentication data ({} bytes)", len);
                    r.skip(len).await?;
                }
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    trace!("invalid CONNACK property: {:?}", p);
                    return Err(RxError::MalformedPacket);
                }
            };
        }

        Ok(Self {
            session_present: connack_flags > 0,
            reason_code: connect_reason_code,
            session_expiry_interval,
            receive_maximum,
            maximum_qos,
            retain_available,
            maximum_packet_size,
            assigned_client_identifier,
            topic_alias_maximum,
            reason_string,
            user_properties,
            wildcard_subscription_available,
            subscription_identifier_available,
            shared_subscription_available,
            server_keep_alive,
            response_information,
            server_reference,
        })
    }
}

#[cfg(test)]
mod unit {
    use core::num::NonZero;

    use crate::{
        config::{KeepAlive, MaximumPacketSize, SessionExpiryInterval},
        test::rx::decode,
        types::{MqttString, MqttStringPair, QoS, ReasonCode},
        v5::{
            packet::ConnackPacket,
            property::{
                AssignedClientIdentifier, MaximumQoS, ReasonString, ReceiveMaximum,
                ResponseInformation, RetainAvailable, ServerKeepAlive, ServerReference,
                SharedSubscriptionAvailable, SubscriptionIdentifierAvailable, TopicAliasMaximum,
                UserProperty, WildcardSubscriptionAvailable,
            },
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn decode_simple() {
        let packet = decode!(ConnackPacket<16>, 3, [0x20, 0x03, 0x01, 0x9D, 0x00]);

        assert_eq!(packet.reason_code, ReasonCode::ServerMoved);
        assert!(packet.session_present);

        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.receive_maximum.is_none());
        assert!(packet.maximum_qos.is_none());
        assert!(packet.retain_available.is_none());
        assert!(packet.maximum_packet_size.is_none());
        assert!(packet.assigned_client_identifier.is_none());
        assert!(packet.topic_alias_maximum.is_none());
        assert!(packet.user_properties.is_empty());
        assert!(packet.reason_string.is_none());
        assert!(packet.wildcard_subscription_available.is_none());
        assert!(packet.subscription_identifier_available.is_none());
        assert!(packet.shared_subscription_available.is_none());
        assert!(packet.server_keep_alive.is_none());
        assert!(packet.response_information.is_none());
        assert!(packet.server_reference.is_none());
        // assert!(packet.authentication_method.is_none());
        // assert!(packet.authentication_data.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(
            ConnackPacket<16>,
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
        assert_eq!(
            packet.receive_maximum,
            Some(ReceiveMaximum(NonZero::new(4761).unwrap()))
        );
        assert_eq!(packet.maximum_qos, Some(MaximumQoS(QoS::AtLeastOnce)));
        assert_eq!(packet.retain_available, Some(RetainAvailable(true)));
        assert_eq!(
            packet.maximum_packet_size,
            Some(MaximumPacketSize::Limit(NonZero::new(1048576).unwrap()))
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
        assert_eq!(packet.user_properties.len(), 1);
        assert_eq!(
            packet.user_properties.first().unwrap(),
            &UserProperty(MqttStringPair::new(
                MqttString::try_from("key").unwrap(),
                MqttString::try_from("value").unwrap()
            ))
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
            Some(ServerKeepAlive(KeepAlive::Seconds(
                NonZero::new(60).unwrap()
            )))
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
        // assert_eq!(
        //     packet.authentication_method,
        //     Some(AuthenticationMethod(
        //         MqttString::try_from("SCRAM-SHA-256").unwrap()
        //     ))
        // );
        // assert_eq!(
        //     packet.authentication_data,
        //     Some(AuthenticationData(
        //         MqttBinary::try_from("auth_data".as_bytes()).unwrap()
        //     ))
        // );
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_incomplete_user_properties() {
        #[rustfmt::skip]
        let packet = decode!(
            ConnackPacket<1>,
            30,
            [
                0x20,
                0x1E,

                0x00, // connect acknowledge flags
                0x00, // reason code
                0x1B, // property length

                // User Property
                0x26, 0x00, 0x02, b'k', b'1',
                      0x00, 0x02, b'v', b'1',

                // User Property
                0x26, 0x00, 0x02, b'k', b'2',
                      0x00, 0x02, b'v', b'2',

                // User Property
                0x26, 0x00, 0x02, b'k', b'3',
                      0x00, 0x02, b'v', b'3',
            ]
        );

        assert_eq!(
            packet.user_properties.first().unwrap(),
            &UserProperty(MqttStringPair::new(
                MqttString::try_from("k1").unwrap(),
                MqttString::try_from("v1").unwrap()
            ))
        );
    }
}
