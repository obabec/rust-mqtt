use heapless::Vec;

use crate::{
    eio::Write,
    header::{FixedHeader, PacketType},
    io::write::Writable,
    packet::{Packet, TxError, TxPacket},
    types::{PacketIdentifier, SubscriptionFilter, TooLargeToEncode, VarByteInt},
    v5::property::{SubscriptionIdentifier, UserProperty},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SubscribePacket<'p, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> {
    packet_identifier: PacketIdentifier,

    subscription_identifier: Option<SubscriptionIdentifier>,
    user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,

    subscribe_filters: Vec<SubscriptionFilter<'p>, MAX_TOPIC_FILTERS>,
}

impl<const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> Packet
    for SubscribePacket<'_, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    const PACKET_TYPE: PacketType = PacketType::Subscribe;
}
impl<const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> TxPacket
    for SubscribePacket<'_, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    fn remaining_len(&self) -> VarByteInt {
        // Safety: SUBSCRIBE packets that are too long to encode cannot be created
        unsafe { self.remaining_len_raw().unwrap_unchecked() }
    }

    /// If `MAX_TOPIC_FILTERS` is to less than or equal to 4095, it is guaranteed that `TxError::RemainingLenExceeded` is never returned.
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x02, self.remaining_len())
            .write(write)
            .await?;

        self.packet_identifier.write(write).await?;
        self.properties_length().write(write).await?;
        self.subscription_identifier.write(write).await?;

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        self.subscribe_filters.write(write).await?;

        Ok(())
    }
}

impl<'p, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize>
    SubscribePacket<'p, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    pub fn new(
        packet_identifier: PacketIdentifier,
        subscription_identifier: Option<SubscriptionIdentifier>,
        user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
        subscribe_filters: Vec<SubscriptionFilter<'p>, MAX_TOPIC_FILTERS>,
    ) -> Result<Self, TooLargeToEncode> {
        let p = Self {
            packet_identifier,
            subscription_identifier,
            user_properties,
            subscribe_filters,
        };

        // Reference to `SubscribePacket::remaining_len_raw` as to why this is true.
        if MAX_USER_PROPERTIES <= 1021 && MAX_TOPIC_FILTERS <= 2053 {
            Ok(p)
        } else {
            p.remaining_len_raw().map(|_| p)
        }
    }

    fn remaining_len_raw(&self) -> Result<VarByteInt, TooLargeToEncode> {
        let variable_header_length = self.packet_identifier.written_len();

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let body_length = self.subscribe_filters.written_len();

        let total_length = variable_header_length + total_properties_length + body_length;

        // max length = MAX_USER_PROPERTIES * 131077 + MAX_TOPIC_FILTERS * 65538 + 11
        // Invariant: The following inequation must be fulfilled to guarantee
        // max length <= VarByteInt::MAX_ENCODABLE:
        //
        // MAX_USER_PROPERTIES * 131077 + MAX_TOPIC_FILTERS * 65538 + 11 <= VarByteInt::MAX_ENCODABLE
        //
        // Given the maximum allowed value of MAX_USER_PROPERTIES = 1021 in the client, the following
        // inequation must be fulfilled:
        //
        // 1021 * 131077 + MAX_TOPIC_FILTERS * 65538 + 11 <= VarByteInt::MAX_ENCODABLE
        //
        // This results in the statement:
        //
        // MAX_TOPIC_FILTERS <= 2053 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // packet identifier: 2
        // property length: 4
        // properties: MAX_USER_PROPERTIES * 131077 + 5
        // topic filters: MAX_TOPIC_FILTERS * 65538
        VarByteInt::try_from(total_length as u32)
    }

    pub fn properties_length(&self) -> VarByteInt {
        let len = self.subscription_identifier.written_len()
            + self
                .user_properties
                .iter()
                .map(Writable::written_len)
                .sum::<usize>();

        // max length = MAX_USER_PROPERTIES * 131077 + 5
        // Invariant: MAX_USER_PROPERTIES <= 2047 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // subscription identifier: 5
        // user properties: MAX_USER_PROPERTIES * 131077
        VarByteInt::new_unchecked(len as u32)
    }
}

#[cfg(test)]
mod unit {
    use core::num::NonZero;

    use heapless::Vec;

    use crate::{
        client::options::{RetainHandling, SubscriptionOptions},
        test::tx::encode,
        types::{
            MqttString, MqttStringPair, PacketIdentifier, SubscriptionFilter, TopicFilter,
            VarByteInt,
        },
        v5::{
            packet::SubscribePacket,
            property::{SubscriptionIdentifier, UserProperty},
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_payload() {
        let topics = [
            SubscriptionFilter::new(
                TopicFilter::new(MqttString::try_from("test/hello").unwrap()).unwrap(),
                &SubscriptionOptions::new().no_local(),
            ),
            SubscriptionFilter::new(
                TopicFilter::new(MqttString::try_from("asdfjklo/#").unwrap()).unwrap(),
                &SubscriptionOptions::new()
                    .retain_handling(RetainHandling::NeverSend)
                    .retain_as_published()
                    .exactly_once(),
            ),
        ];
        let packet: SubscribePacket<'_, 2, 0> = SubscribePacket::new(
            PacketIdentifier::new(NonZero::new(23197).unwrap()),
            None,
            Vec::new(),
            topics.into(),
        )
        .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
                0x82, //
                0x1D, // remaining length
                0x5A, // Packet identifier MSB
                0x9D, // Packet identifier LSB
                0x00, // Property length
                0x00, // Payload - Topic Filter
                0x0A, // |
                b't', // |
                b'e', // |
                b's', // |
                b't', // |
                b'/', // |
                b'h', // |
                b'e', // |
                b'l', // |
                b'l', // |
                b'o', // Payload - Topic Filter
                0x04, // Payload - Subscription Options
                0x00, // Payload - Topic Filter
                0x0A, // |
                b'a', // |
                b's', // |
                b'd', // |
                b'f', // |
                b'j', // |
                b'k', // |
                b'l', // |
                b'o', // |
                b'/', // |
                b'#', // Payload - Topic Filter
                0x2A, // Payload - Subscription Options
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_properties() {
        let topics = [SubscriptionFilter::new(
            TopicFilter::new(MqttString::try_from("abc/+/y").unwrap()).unwrap(),
            &SubscriptionOptions::new()
                .retain_handling(RetainHandling::SendIfNotSubscribedBefore)
                .retain_as_published()
                .subscription_identifier(VarByteInt::from(23459u16)),
        )];

        let user_properties = [
            UserProperty(MqttStringPair::new(
                MqttString::from_str("answer").unwrap(),
                MqttString::from_str("42").unwrap(),
            )),
            UserProperty(MqttStringPair::new(
                MqttString::from_str("chaos").unwrap(),
                MqttString::from_str("maximum").unwrap(),
            )),
        ];

        let packet: SubscribePacket<'_, 10, 16> = SubscribePacket::new(
            PacketIdentifier::new(NonZero::new(23197).unwrap()),
            Some(SubscriptionIdentifier(
                VarByteInt::new(87986078u32).unwrap(),
            )),
            user_properties.into(),
            topics.into(),
        )
        .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
                0x82, //
                0x30, // remaining length
                0x5A, // Packet identifier MSB
                0x9D, // Packet identifier LSB
                0x23, // Property length

                // Subscription Identifier
                0x0B, 0x9E, 0x9F, 0xFA, 0x29,

                // User Property
                0x26,
                0x00, 0x06, b'a', b'n', b's', b'w', b'e', b'r',
                0x00, 0x02, b'4', b'2',

                // User Property
                0x26,
                0x00, 0x05, b'c', b'h', b'a', b'o', b's',
                0x00, 0x07, b'm', b'a', b'x', b'i', b'm', b'u', b'm',
                
                // Payload - Topic Filter
                0x00, 0x07, b'a', b'b', b'c', b'/', b'+', b'/', b'y', 0x18,
            ]
        );
    }
}
