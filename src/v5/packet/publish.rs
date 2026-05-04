use heapless::Vec;

use crate::{
    buffer::BufferProvider,
    bytes::Bytes,
    client::options::TopicReference,
    eio::{Read, Write},
    fmt::{trace, verbose},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable, Store},
        write::Writable,
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{
        IdentifiedQoS, MqttBinary, MqttString, PacketIdentifier, QoS, TooLargeToEncode, TopicName,
        VarByteInt,
    },
    v5::property::{
        AtMostOnceProperty, ContentType, CorrelationData, MessageExpiryInterval,
        PayloadFormatIndicator, Property, PropertyType, ResponseTopic, SubscriptionIdentifier,
        TopicAlias, UserProperty,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PublishPacket<
    'p,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
> {
    pub dup: bool,
    pub identified_qos: IdentifiedQoS,
    pub retain: bool,

    pub topic: TopicReference<'p>,

    // TODO clarify whether PayloadFormatIndicator can be included only once
    pub payload_format_indicator: Option<PayloadFormatIndicator>,

    // TODO clarify whether MessageExpiryInterval can be included only once
    pub message_expiry_interval: Option<MessageExpiryInterval>,
    pub response_topic: Option<ResponseTopic<'p>>,
    pub correlation_data: Option<CorrelationData<'p>>,
    pub user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
    pub subscription_identifiers: Vec<SubscriptionIdentifier, MAX_SUBSCRIPTION_IDENTIFIERS>,
    pub content_type: Option<ContentType<'p>>,
    pub message: Bytes<'p>,
}

impl<const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize> Packet
    for PublishPacket<'_, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>
{
    const PACKET_TYPE: PacketType = PacketType::Publish;
}
impl<'p, const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize> RxPacket<'p>
    for PublishPacket<'p, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>
{
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding PUBLISH packet");

        let flags = header.flags();

        verbose!("decoding PUBLISH flags");
        let dup = flags >> 3 == 1;
        let qos = QoS::try_from_bits((flags >> 1) & 0x03).ok_or(RxError::MalformedPacket)?;
        let retain = flags & 0x01 == 1;

        let r = &mut reader;

        verbose!("reading topic name field");
        let topic_name = MqttString::read(r).await?;

        let topic_name = if topic_name.is_empty() {
            None
        } else {
            Some(TopicName::new(topic_name).ok_or(RxError::InvalidTopicName)?)
        };

        let identified_qos = match qos {
            QoS::AtMostOnce => IdentifiedQoS::AtMostOnce,
            QoS::AtLeastOnce => {
                verbose!("reading packet identifier field");
                IdentifiedQoS::AtLeastOnce(PacketIdentifier::read(r).await?)
            }
            QoS::ExactlyOnce => {
                verbose!("reading packet identifier field");
                IdentifiedQoS::ExactlyOnce(PacketIdentifier::read(r).await?)
            }
        };

        verbose!("reading property length field");
        let mut properties_length = VarByteInt::read(r).await?.size();
        verbose!("property length: {} bytes", properties_length);

        let mut payload_format_indicator: Option<PayloadFormatIndicator> = None;
        let mut message_expiry_interval: Option<MessageExpiryInterval> = None;
        let mut topic_alias: Option<TopicAlias> = None;
        let mut response_topic: Option<ResponseTopic<'_>> = None;
        let mut correlation_data: Option<CorrelationData<'_>> = None;
        let mut user_properties = Vec::new();
        let mut subscription_identifiers = Vec::new();
        let mut content_type: Option<ContentType<'_>> = None;

        while properties_length > 0 {
            verbose!(
                "reading property identifier (remaining length: {} bytes)",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;

            // unchecked sub because `properties_length` > 0
            properties_length -= property_type.written_len();

            verbose!(
                "reading {:?} property body (remaining length: {} bytes)",
                property_type,
                r.remaining_len()
            );
            match property_type {
                PropertyType::PayloadFormatIndicator => {
                    payload_format_indicator.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(payload_format_indicator.unwrap().into_inner().written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::MessageExpiryInterval => {
                    message_expiry_interval.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(message_expiry_interval.unwrap().into_inner().written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::TopicAlias => {
                    topic_alias.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(topic_alias.unwrap().into_inner().written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::ResponseTopic => {
                    response_topic.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(response_topic.as_ref().unwrap().0.written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::CorrelationData => {
                    correlation_data.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(correlation_data.as_ref().unwrap().0.written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::UserProperty if !user_properties.is_full() => {
                    let user_property = UserProperty::read(r).await?;

                    properties_length = properties_length
                        .checked_sub(user_property.0.written_len())
                        .ok_or(RxError::MalformedPacket)?;

                    // Safety: `!Vec::is_full` guarantees there is space
                    unsafe { user_properties.push_unchecked(user_property) };
                }
                PropertyType::UserProperty => {
                    let len = UserProperty::skip(r).await?;
                    properties_length = properties_length
                        .checked_sub(len)
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::SubscriptionIdentifier => {
                    let subscription_identifier = SubscriptionIdentifier::read(r).await?;

                    // The subscription identifiers in the packet are not guaranteed to be exhaustive
                    #[allow(unused_must_use)]
                    subscription_identifiers.push(subscription_identifier);
                    properties_length = properties_length
                        .checked_sub(subscription_identifier.into_inner().written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::ContentType => {
                    content_type.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(content_type.as_ref().unwrap().0.written_len())
                        .ok_or(RxError::MalformedPacket)?;
                }
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    trace!("invalid PUBLISH property: {:?}", p);
                    return Err(RxError::MalformedPacket);
                }
            }
        }

        let topic = match (topic_name, topic_alias) {
            (None, None) => return Err(RxError::ProtocolError),
            (None, Some(alias)) => TopicReference::Alias(alias.into_inner()),
            (Some(name), None) => TopicReference::Name(name),
            (Some(name), Some(alias)) => TopicReference::Mapping(name, alias.into_inner()),
        };

        let message_len = r.remaining_len();

        verbose!("reading PUBLISH payload ({} bytes)", message_len);

        let message = r.read_and_store(r.remaining_len()).await?;

        Ok(Self {
            dup,
            identified_qos,
            retain,
            topic,
            payload_format_indicator,
            message_expiry_interval,
            response_topic,
            correlation_data,
            user_properties,
            subscription_identifiers,
            content_type,
            message,
        })
    }
}
impl<const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize> TxPacket
    for PublishPacket<'_, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>
{
    fn remaining_len(&self) -> VarByteInt {
        // Safety: PUBLISH packets that are too long to encode cannot be created
        unsafe { self.remaining_len_raw().unwrap_unchecked() }
    }

    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        let qos: QoS = self.identified_qos.into();
        let flags = (u8::from(self.dup) << 3) | qos.into_bits(1) | u8::from(self.retain);

        FixedHeader::new(Self::PACKET_TYPE, flags, self.remaining_len())
            .write(write)
            .await?;

        self.topic
            .topic_name()
            .map(TopicName::as_borrowed)
            .map_or(Self::EMPTY_TOPIC, Into::into)
            .write(write)
            .await?;

        if let Some(p) = self.identified_qos.packet_identifier() {
            p.write(write).await?;
        }

        self.properties_length().write(write).await?;
        self.payload_format_indicator.write(write).await?;
        self.message_expiry_interval.write(write).await?;
        self.topic.alias().map(TopicAlias).write(write).await?;
        self.response_topic.write(write).await?;
        self.correlation_data.write(write).await?;

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        // Don't write subscription identifiers as they are irrational when publishing from client to server
        self.content_type.write(write).await?;

        self.message.write(write).await?;

        Ok(())
    }
}

impl<'p, const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize>
    PublishPacket<'p, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>
{
    // Invariant: Empty string does not exceed MqttString::MAX_LENGTH
    const EMPTY_TOPIC: MqttString<'static> = MqttString::from_str_unchecked("");

    /// Creates a new packet with Quality of Service 0
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dup: bool,
        identified_qos: IdentifiedQoS,
        retain: bool,
        topic: TopicReference<'p>,
        payload_format_indicator: Option<PayloadFormatIndicator>,
        message_expiry_interval: Option<MessageExpiryInterval>,
        response_topic: Option<TopicName<'p>>,
        correlation_data: Option<MqttBinary<'p>>,
        user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
        content_type: Option<ContentType<'p>>,
        message: Bytes<'p>,
    ) -> Result<Self, TooLargeToEncode> {
        let p = Self {
            dup,
            identified_qos,
            retain,
            topic,
            payload_format_indicator,
            message_expiry_interval,
            response_topic: response_topic.map(Into::into),
            correlation_data: correlation_data.map(Into::into),
            user_properties,
            subscription_identifiers: Vec::new(),
            content_type,
            message,
        };

        p.remaining_len_raw().map(|_| p)
    }

    fn remaining_len_raw(&self) -> Result<VarByteInt, TooLargeToEncode> {
        let topic_name_length = self
            .topic
            .topic_name()
            .map(TopicName::as_borrowed)
            .map_or(Self::EMPTY_TOPIC, Into::into)
            .written_len();

        let variable_header_length = topic_name_length
            + self
                .identified_qos
                .packet_identifier()
                .as_ref()
                .map(Writable::written_len)
                .unwrap_or_default();

        let properties_length = self.properties_length().ok_or(TooLargeToEncode)?;
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let body_length = self.message.len();

        let total_length = variable_header_length + total_properties_length + body_length;

        // max length = MAX_USER_PROPERTIES * 131077 + 262,167 + MAX_MESSAGE_LENGTH
        //
        // topic name: 65537
        // packet identifier: 2
        // property length: 4
        // properties: MAX_USER_PROPERTIES * 131077 + 196624
        // message: MAX_MESSAGE_LENGTH
        VarByteInt::try_from(total_length as u32)
    }

    fn properties_length(&self) -> Option<VarByteInt> {
        let len = self.payload_format_indicator.written_len()
            + self.message_expiry_interval.written_len()
            + self.topic.alias().map(TopicAlias).written_len()
            + self.response_topic.written_len()
            + self.correlation_data.written_len()
            + self
                .user_properties
                .iter()
                .map(Writable::written_len)
                .sum::<usize>()
            + self.content_type.written_len();

        // max length = MAX_USER_PROPERTIES * 131077 + 196624
        // Invariant: MAX_USER_PROPERTIES <= 2046 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // payload format indicator: 2
        // message expiry interval: 5
        // topic alias: 3
        // response topic: 65538
        // correlation data: 65538
        // user properties: MAX_USER_PROPERTIES * 131077
        // no subscription identifiers in client to server publish
        // content type: 65538
        VarByteInt::new(len as u32)
    }
}

#[cfg(test)]
mod unit {
    use core::num::NonZero;

    use heapless::Vec;

    use crate::{
        bytes::Bytes,
        client::options::TopicReference,
        test::{rx::decode, tx::encode},
        types::{
            IdentifiedQoS, MqttBinary, MqttString, MqttStringPair, PacketIdentifier, TopicName,
            VarByteInt,
        },
        v5::{
            packet::PublishPacket,
            property::{
                ContentType, CorrelationData, MessageExpiryInterval, PayloadFormatIndicator,
                ResponseTopic, UserProperty,
            },
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_simple() {
        let packet: PublishPacket<'_, 0, 0> = PublishPacket::new(
            false,
            IdentifiedQoS::AtLeastOnce(PacketIdentifier::new(NonZero::new(5897).unwrap())),
            false,
            TopicReference::Name(
                TopicName::new(MqttString::try_from("test/topic").unwrap()).unwrap(),
            ),
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            Bytes::from("hello".as_bytes()),
        )
        .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
            0x32,
            0x14,
            0x00, // Topic Name
            0x0A, //
            b't', //
            b'e', //
            b's', //
            b't', //
            b'/', //
            b't', //
            b'o', //
            b'p', //
            b'i', //
            b'c', // Topic Name
            0x17, // Packet identifier
            0x09, // Packet identifier
            0x00, // Property length
            b'h', // Payload
            b'e', //
            b'l', //
            b'l', //
            b'o', // Payload
        ]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_properties() {
        let packet: PublishPacket<'_, 0, 16> = PublishPacket::new(
            true,
            IdentifiedQoS::ExactlyOnce(PacketIdentifier::new(NonZero::new(9624).unwrap())),
            true,
            TopicReference::Alias(NonZero::new(23408).unwrap()),
            Some(false.into()),
            Some(481123u32.into()),
            Some(TopicName::new(MqttString::from_str("uno, dos, tres, catorce").unwrap()).unwrap()),
            Some(MqttBinary::from_slice_unchecked(&[0, 1, 2, 3, 4, 5, 6, 7])),
            [
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("donald").unwrap(),
                    MqttString::from_str("duck").unwrap(),
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("Gyro").unwrap(),
                    MqttString::from_str("Gearloose").unwrap(),
                )),
            ]
            .into(),
            Some(
                MqttString::from_str("application/javascript")
                    .unwrap()
                    .into(),
            ),
            Bytes::from("hello".as_bytes()),
        )
        .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
            0x3D,
            0x73,
            0x00, // Topic Name
            0x00, // Topic Name
            0x25, // Packet identifier
            0x98, // Packet identifier
            0x69, // Property length

            0x01, // Payload format indicator
            0x00, // Payload format indicator
            0x02, // Message expiry interval
            0x00, //
            0x07, //
            0x57, //
            0x63, // Message expiry interval
            0x23, // Topic alias
            0x5B, //
            0x70, // Topic alias

            0x08, // Response Topic
            0x00, 0x17,
            b'u', b'n', b'o', b',', b' ', b'd', b'o', b's', b',', b' ', b't', b'r', b'e', b's', b',', b' ', b'c', b'a', b't', b'o', b'r', b'c', b'e', 
            
            0x09, // Correlation Data
            0x00, 0x08,
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,

            0x26, // User property
            0x00, 0x06, b'd', b'o', b'n', b'a', b'l', b'd',
            0x00, 0x04, b'd', b'u', b'c', b'k',

            0x26, // User property
            0x00, 0x04, b'G', b'y', b'r', b'o',
            0x00, 0x09, b'G', b'e', b'a', b'r', b'l', b'o', b'o', b's', b'e', 

            0x03, // Content type
            0x00, 0x16,
            b'a', b'p', b'p', b'l', b'i', b'c', b'a', b't', b'i', b'o', b'n', b'/', b'j', b'a', b'v', b'a', b's', b'c', b'r', b'i', b'p', b't', 

            b'h', // Payload
            b'e', //
            b'l', //
            b'l', //
            b'o', // Payload
        ]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_simple() {
        let packet = decode!(
            PublishPacket<'_, 0, 16>,
            13,
            [
                0x30, 0x0D, 0x00, 0x0A, b't', b'e', b's', b't', b'/', b't', b'o', b'p', b'i', b'c',
                0x00
            ]
        );

        assert_eq!(packet.identified_qos, IdentifiedQoS::AtMostOnce);
        assert!(!packet.dup);
        assert!(!packet.retain);
        assert_eq!(
            packet.topic,
            TopicReference::Name(
                TopicName::new(MqttString::try_from("test/topic").unwrap()).unwrap()
            )
        );

        assert!(packet.payload_format_indicator.is_none());
        assert!(packet.message_expiry_interval.is_none());
        assert!(packet.response_topic.is_none());
        assert!(packet.correlation_data.is_none());
        assert!(packet.user_properties.is_empty());
        assert!(packet.subscription_identifiers.is_empty());
        assert!(packet.content_type.is_none());

        assert_eq!(packet.message, Bytes::from([].as_slice()));
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_payload() {
        let packet = decode!(
            PublishPacket<'_, 0, 16>,
            21,
            [
                0x3D, 0x15, 0x00, 0x04, b't', b'e', b's', b't', 0x54, 0x23, 0x00, b'h', b'e', b'l',
                b'l', b'o', b',', b' ', b't', b'h', b'e', b'r', b'e',
            ]
        );

        assert_eq!(
            packet.identified_qos,
            IdentifiedQoS::ExactlyOnce(PacketIdentifier::new(NonZero::new(21539).unwrap()))
        );
        assert!(packet.dup);
        assert!(packet.retain);
        assert_eq!(
            packet.topic,
            TopicReference::Name(TopicName::new(MqttString::try_from("test").unwrap()).unwrap())
        );
        assert!(packet.payload_format_indicator.is_none());
        assert!(packet.message_expiry_interval.is_none());
        assert!(packet.response_topic.is_none());
        assert!(packet.correlation_data.is_none());
        assert!(packet.user_properties.is_empty());
        assert!(packet.subscription_identifiers.is_empty());
        assert!(packet.content_type.is_none());

        assert_eq!(packet.message, Bytes::from("hello, there".as_bytes()));
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(
            PublishPacket<'_, 1, 16>,
            79,
            [
                0x30, 0x4F,

                0x00, 0x04, b't', b'e', b's', b't', // Topic name "test"
                0x43, // Property length

                // Payload Format Indicator
                0x01, 0x01,

                // Message Expiry Interval
                0x02, 0x00, 0x00, 0x1C, 0x20,

                // Topic Alias
                0x23, 0x00, 0x0A,

                // Response Topic
                0x08, 0x00, 0x0E, b'r', b'e', b's', b'p', b'o', b'n', b's', b'e', b'/', b't', b'o', b'p', b'i', b'c',

                // Correlation Data
                0x09, 0x00, 0x08, b'c', b'o', b'r', b'r', b'_', b'i', b'd', b'1',

                // User Property
                0x26, 0x00, 0x04, b'n', b'a', b'm', b'e', 0x00, 0x05, b'v', b'a', b'l', b'u', b'e',

                // Subscription Identifier
                0x0B, 0x2A,

                // Content Type
                0x03, 0x00, 0x0A, b't', b'e', b'x', b't', b'/', b'p', b'l', b'a', b'i', b'n',

                // Payload
                b'h', b'e', b'l', b'l', b'o',
            ]
        );

        assert_eq!(packet.identified_qos, IdentifiedQoS::AtMostOnce);
        assert!(!packet.dup);
        assert!(!packet.retain);
        assert_eq!(
            packet.topic,
            TopicReference::Mapping(
                TopicName::new(MqttString::try_from("test").unwrap()).unwrap(),
                NonZero::new(10).unwrap(),
            )
        );
        assert_eq!(packet.message, Bytes::from("hello".as_bytes()));
        assert_eq!(
            packet.payload_format_indicator,
            Some(PayloadFormatIndicator(true))
        );
        assert_eq!(
            packet.message_expiry_interval,
            Some(MessageExpiryInterval(7200))
        );

        assert_eq!(
            packet.response_topic,
            Some(ResponseTopic(
                TopicName::new(MqttString::try_from("response/topic").unwrap()).unwrap()
            ))
        );
        assert_eq!(
            packet.correlation_data,
            Some(CorrelationData(
                MqttBinary::try_from("corr_id1".as_bytes()).unwrap()
            ))
        );

        assert_eq!(
            packet.user_properties.as_slice(),
            &[UserProperty(MqttStringPair::new(
                MqttString::from_str("name").unwrap(),
                MqttString::from_str("value").unwrap()
            ))]
        );

        assert_eq!(
            packet.subscription_identifiers.as_slice(),
            &[VarByteInt::from(42u8).into()]
        );

        assert_eq!(
            packet.content_type,
            Some(ContentType(MqttString::try_from("text/plain").unwrap()))
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_incomplete_subscription_identifiers() {
        #[rustfmt::skip]
        let packet = decode!(
            PublishPacket<'_, 2, 16>,
            21,
            [
                0x30,
                0x15,

                0x00, 0x04, b't', b'e', b's', b't', // Topic name "test"
                0x0C, // Property length

                // Subscription Identifier
                0x0B, 0x80, 0xAD, 0xE2, 0x04,

                // Subscription Identifier
                0x0B, 0xF4, 0x03,

                // Subscription Identifier
                0x0B, 0xA0, 0x9C, 0x01,

                // Payload
                b'O', b'K',
            ]
        );

        assert_eq!(
            packet.subscription_identifiers.as_slice(),
            &[
                VarByteInt::try_from(10_000_000u32).unwrap().into(),
                VarByteInt::from(500u16).into()
            ]
        );
        assert_eq!(packet.message, Bytes::from("OK".as_bytes()));
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_incomplete_user_properties() {
        #[rustfmt::skip]
        let packet = decode!(
            PublishPacket<'_, 1, 1>,
            27,
            [
                0x30,
                0x1B,

                0x00, 0x04, b't', b'e', b's', b't', // Topic name "test"
                0x12, // Property length

                // User Property
                0x26, 0x00, 0x02, b'k', b'1',
                      0x00, 0x02, b'v', b'1',

                // User Property
                0x26, 0x00, 0x02, b'k', b'2',
                      0x00, 0x02, b'v', b'2',

                // Payload
                b'o', b'k',
            ]
        );

        assert_eq!(
            packet.user_properties.as_slice(),
            &[UserProperty(MqttStringPair::new(
                MqttString::from_str("k1").unwrap(),
                MqttString::from_str("v1").unwrap()
            ))]
        );
        assert_eq!(packet.message, Bytes::from("ok".as_bytes()));
    }
}
