use crate::{
    buffer::BufferProvider,
    bytes::Bytes,
    eio::{Read, Write},
    fmt::{error, trace},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable, Store},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{IdentifiedQoS, MqttString, QoS, TooLargeToEncode, VarByteInt},
    v5::property::{
        AtMostOnceProperty, ContentType, CorrelationData, MessageExpiryInterval,
        PayloadFormatIndicator, Property, PropertyType, ResponseTopic, SubscriptionIdentifier,
        TopicAlias,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PublishPacket<'p> {
    pub dup: bool,
    pub identified_qos: IdentifiedQoS,
    pub retain: bool,

    pub topic: MqttString<'p>,

    // TODO clarify whether PayloadFormatIndicator can be included only once
    pub payload_format_indicator: Option<PayloadFormatIndicator>,

    // TODO clarify whether MessageExpiryInterval can be included only once
    pub message_expiry_interval: Option<MessageExpiryInterval>,
    pub topic_alias: Option<TopicAlias>,
    pub response_topic: Option<ResponseTopic<'p>>,
    pub correlation_data: Option<CorrelationData<'p>>,
    // TODO subscription identifiers
    pub content_type: Option<ContentType<'p>>,
    pub message: Bytes<'p>,
}

impl<'p> Packet for PublishPacket<'p> {
    const PACKET_TYPE: PacketType = PacketType::Publish;
}
impl<'p> RxPacket<'p> for PublishPacket<'p> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding");

        let flags = header.flags();

        trace!("decoding flags");
        let dup = flags >> 3 == 1;
        let qos = QoS::try_from_bits((flags >> 1) & 0x03).map_err(|_| RxError::MalformedPacket)?;
        let retain = flags & 0x01 == 1;

        let r = &mut reader;

        trace!("reading topic name");
        let topic_name = MqttString::read(r).await?;

        let identified_qos = match qos {
            QoS::AtMostOnce => IdentifiedQoS::AtMostOnce,
            QoS::AtLeastOnce => {
                trace!("reading packet identifier");
                IdentifiedQoS::AtLeastOnce(u16::read(r).await?)
            }
            QoS::ExactlyOnce => {
                trace!("reading packet identifier");
                IdentifiedQoS::ExactlyOnce(u16::read(r).await?)
            }
        };

        trace!("reading properties length");
        let mut properties_length = VarByteInt::read(r).await?.size();
        trace!("properties length = {}", properties_length);

        let mut payload_format_indicator: Option<PayloadFormatIndicator> = None;
        let mut message_expiry_interval: Option<MessageExpiryInterval> = None;
        let mut topic_alias: Option<TopicAlias> = None;
        let mut response_topic: Option<ResponseTopic<'_>> = None;
        let mut correlation_data: Option<CorrelationData<'_>> = None;
        let mut content_type: Option<ContentType<'_>> = None;

        while properties_length > 0 {
            trace!(
                "reading property type with remaining len = {}",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;
            properties_length = properties_length
                .checked_sub(property_type.written_len())
                .ok_or(RxError::MalformedPacket)?;

            trace!(
                "reading property body of {:?} with remaining len = {}",
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
                #[rustfmt::skip]
                PropertyType::UserProperty => {
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    properties_length = properties_length.checked_sub(wlen!(u16) + len).ok_or(RxError::MalformedPacket)?;
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    properties_length = properties_length.checked_sub(wlen!(u16) + len).ok_or(RxError::MalformedPacket)?;
                }
                PropertyType::SubscriptionIdentifier => {
                    // Since multiple subscription identifiers can appear, we just parse it into the void for now
                    let mut subscription_identifier: Option<SubscriptionIdentifier> = None;
                    subscription_identifier.try_set(r).await?;
                    properties_length = properties_length
                        .checked_sub(subscription_identifier.unwrap().into_inner().written_len())
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
                    error!("packet contains unexpected property {:?}", p);
                    return Err(RxError::MalformedPacket);
                }
            };
        }

        let message_len = r.remaining_len();

        trace!("reading message ({} bytes)", message_len);

        let payload = r.read_and_store(r.remaining_len()).await?;

        Ok(PublishPacket {
            dup,
            identified_qos,
            retain,
            topic: topic_name,
            payload_format_indicator,
            message_expiry_interval,
            topic_alias,
            response_topic,
            correlation_data,
            content_type,
            message: payload,
        })
    }
}
impl<'p> TxPacket for PublishPacket<'p> {
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        // Safety: Publish packets that are too long to encode cannot be created
        let remaining_len = unsafe { self.remaining_length().unwrap_unchecked() };

        let qos: QoS = self.identified_qos.into();
        let flags = (u8::from(self.dup) << 3) | qos.into_bits(1) | u8::from(self.retain);

        FixedHeader::new(Self::PACKET_TYPE, flags, remaining_len)
            .write(write)
            .await?;

        self.topic.write(write).await?;
        if let Some(p) = self.identified_qos.packet_identifier() {
            p.write(write).await?
        }

        self.properties_length().write(write).await?;
        self.payload_format_indicator.write(write).await?;
        self.message_expiry_interval.write(write).await?;
        self.topic_alias.write(write).await?;
        self.response_topic.write(write).await?;
        self.correlation_data.write(write).await?;
        self.content_type.write(write).await?;

        self.message.write(write).await?;

        Ok(())
    }
}

impl<'p> PublishPacket<'p> {
    /// Creates a new packet with Quality of Service 0
    pub fn new(
        dup: bool,
        retain: bool,
        identified_qos: IdentifiedQoS,
        topic: MqttString<'p>,
        message: Bytes<'p>,
    ) -> Result<Self, TooLargeToEncode> {
        let p = Self {
            dup,
            identified_qos,
            retain,
            topic,
            payload_format_indicator: None,
            message_expiry_interval: None,
            topic_alias: None,
            response_topic: None,
            correlation_data: None,
            content_type: None,
            message,
        };

        p.remaining_length().map(|_| p)
    }

    fn remaining_length(&self) -> Result<VarByteInt, TooLargeToEncode> {
        let variable_header_length = self.topic.written_len()
            + self
                .identified_qos
                .packet_identifier()
                .map(|p| p.written_len())
                .unwrap_or_default();

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let body_length = self.message.len();

        let total_length = variable_header_length + total_properties_length + body_length;

        VarByteInt::try_from(total_length as u32)
    }

    fn properties_length(&self) -> VarByteInt {
        let len = self.payload_format_indicator.written_len()
            + self.message_expiry_interval.written_len()
            + self.topic_alias.written_len()
            + self.response_topic.written_len()
            + self.correlation_data.written_len()
            + self.content_type.written_len();

        // Safety: Max length = 196624 < VarByteInt::MAX_ENCODABLE
        // payload format indicator: 2
        // message expiry interval: 5
        // topic alias: 3
        // response topic: 65538
        // correlation data: 65538
        // content type: 65538
        unsafe { VarByteInt::new_unchecked(len as u32) }
    }
}

#[cfg(test)]
mod unit {
    use crate::{
        bytes::Bytes,
        test::{rx::decode, tx::encode},
        types::{IdentifiedQoS, MqttBinary, MqttString},
        v5::{
            packet::PublishPacket,
            property::{
                ContentType, CorrelationData, MessageExpiryInterval, PayloadFormatIndicator,
                ResponseTopic, TopicAlias,
            },
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_simple() {
        let packet = PublishPacket::new(
            false,
            false,
            IdentifiedQoS::AtLeastOnce(5897),
            MqttString::try_from("test/topic").unwrap(),
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
    async fn decode_simple() {
        let packet = decode!(
            PublishPacket,
            13,
            [
                0x30, 0x0D, 0x00, 0x0A, b't', b'e', b's', b't', b'/', b't', b'o', b'p', b'i', b'c',
                0x00
            ]
        );

        assert_eq!(packet.identified_qos, IdentifiedQoS::AtMostOnce);
        assert!(!packet.dup);
        assert!(!packet.retain);
        assert_eq!(packet.topic, MqttString::try_from("test/topic").unwrap());

        assert!(packet.payload_format_indicator.is_none());
        assert!(packet.message_expiry_interval.is_none());
        assert!(packet.topic_alias.is_none());
        assert!(packet.response_topic.is_none());
        assert!(packet.correlation_data.is_none());
        assert!(packet.content_type.is_none());

        assert_eq!(packet.message, Bytes::from([].as_slice()));
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_payload() {
        let packet = decode!(
            PublishPacket,
            21,
            [
                0x3D, 0x15, 0x00, 0x04, b't', b'e', b's', b't', 0x54, 0x23, 0x00, b'h', b'e', b'l',
                b'l', b'o', b',', b' ', b't', b'h', b'e', b'r', b'e',
            ]
        );

        assert_eq!(packet.identified_qos, IdentifiedQoS::ExactlyOnce(21539));
        assert!(packet.dup);
        assert!(packet.retain);
        assert_eq!(packet.topic, MqttString::try_from("test").unwrap());
        assert!(packet.payload_format_indicator.is_none());
        assert!(packet.message_expiry_interval.is_none());
        assert!(packet.topic_alias.is_none());
        assert!(packet.response_topic.is_none());
        assert!(packet.correlation_data.is_none());
        assert!(packet.content_type.is_none());

        assert_eq!(packet.message, Bytes::from("hello, there".as_bytes()));
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(
            PublishPacket,
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
        assert_eq!(packet.topic, MqttString::try_from("test").unwrap());
        assert_eq!(packet.message, Bytes::from("hello".as_bytes()));
        assert_eq!(
            packet.payload_format_indicator,
            Some(PayloadFormatIndicator(true))
        );
        assert_eq!(
            packet.message_expiry_interval,
            Some(MessageExpiryInterval(7200))
        );
        assert_eq!(packet.topic_alias, Some(TopicAlias(10)));

        assert_eq!(
            packet.response_topic,
            Some(ResponseTopic(
                MqttString::try_from("response/topic").unwrap()
            ))
        );
        assert_eq!(
            packet.correlation_data,
            Some(CorrelationData(
                MqttBinary::try_from("corr_id1".as_bytes()).unwrap()
            ))
        );

        // TODO subscription identifier = 42u32

        assert_eq!(
            packet.content_type,
            Some(ContentType(MqttString::try_from("text/plain").unwrap()))
        );
    }
}
