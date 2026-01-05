use heapless::Vec;

use crate::{
    eio::Write,
    header::{FixedHeader, PacketType},
    io::write::Writable,
    packet::{Packet, TxError, TxPacket},
    types::{SubscriptionFilter, TooLargeToEncode, VarByteInt},
    v5::property::SubscriptionIdentifier,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SubscribePacket<'p, const MAX_TOPIC_FILTERS: usize> {
    packet_identifier: u16,

    subscription_identifier: Option<SubscriptionIdentifier>,
    subscribe_filters: Vec<SubscriptionFilter<'p>, MAX_TOPIC_FILTERS>,
}

impl<'p, const MAX_TOPIC_FILTERS: usize> Packet for SubscribePacket<'p, MAX_TOPIC_FILTERS> {
    const PACKET_TYPE: PacketType = PacketType::Subscribe;
}
impl<'p, const MAX_TOPIC_FILTERS: usize> TxPacket for SubscribePacket<'p, MAX_TOPIC_FILTERS> {
    fn remaining_len(&self) -> VarByteInt {
        // Safety: SUBSCRIBE packets that are too long to encode cannot be created
        unsafe { self.remaining_len_raw().unwrap_unchecked() }
    }

    /// If MAX_TOPIC_FILTERS is to less than or equal to 4095, it is guaranteed that TxError::RemainingLenExceeded is never returned.
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x02, self.remaining_len())
            .write(write)
            .await?;

        self.packet_identifier.write(write).await?;
        self.properties_length().write(write).await?;
        self.subscription_identifier.write(write).await?;
        self.subscribe_filters.write(write).await?;

        Ok(())
    }
}

impl<'p, const MAX_TOPIC_FILTERS: usize> SubscribePacket<'p, MAX_TOPIC_FILTERS> {
    /// Creates a new packet with no subscription identifier
    ///
    /// It is up to the caller to verify that the topic filters are short enough
    /// so that the packets complete length doesn't exceed `VarByteInt::MAX_ENCODABLE`.
    pub fn new(
        packet_identifier: u16,
        subscribe_filters: Vec<SubscriptionFilter<'p>, MAX_TOPIC_FILTERS>,
    ) -> Result<Self, TooLargeToEncode> {
        let p = Self {
            packet_identifier,
            subscription_identifier: None,
            subscribe_filters,
        };

        const GUARANTEED_ENCODABLE_TOPIC_FILTERS: usize = 4095;

        if MAX_TOPIC_FILTERS > GUARANTEED_ENCODABLE_TOPIC_FILTERS {
            p.remaining_len_raw().map(|_| p)
        } else {
            Ok(p)
        }
    }

    fn remaining_len_raw(&self) -> Result<VarByteInt, TooLargeToEncode> {
        let variable_header_length = self.packet_identifier.written_len();

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let body_length = self.subscribe_filters.written_len();

        let total_length = variable_header_length + total_properties_length + body_length;

        // MAX_TOPIC_FILTERS has to be less than or equal to 4095 to guarantee:
        //   Max length = 11 + MAX_TOPIC_FILTERS * 65538 <= VarByteInt::MAX_ENCODABLE
        // packet identifier: 2
        // properties length: 4
        // properties: 5
        // topic filters: MAX_TOPIC_FILTERS * 65538
        VarByteInt::try_from(total_length as u32)
    }

    #[cfg(test)]
    pub fn add_subscription_identifier(
        &mut self,
        subscription_identifier: VarByteInt,
    ) -> Result<(), TooLargeToEncode> {
        let previous = self
            .subscription_identifier
            .replace(subscription_identifier.into());

        match self.remaining_len_raw() {
            Ok(_) => Ok(()),
            Err(t) => {
                self.subscription_identifier = previous;
                Err(t)
            }
        }
    }

    pub fn properties_length(&self) -> VarByteInt {
        let len = self.subscription_identifier.written_len();

        // Safety: Max length = 5 < VarByteInt::MAX_ENCODABLE
        // subscription identifier: 5
        unsafe { VarByteInt::new_unchecked(len as u32) }
    }
}

#[cfg(test)]
mod unit {
    use heapless::Vec;

    use crate::{
        client::options::{RetainHandling, SubscriptionOptions},
        test::tx::encode,
        types::{MqttString, QoS, SubscriptionFilter, TopicFilter, VarByteInt},
        v5::packet::SubscribePacket,
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_payload() {
        let mut topics = Vec::new();

        topics
            .push(SubscriptionFilter::new(
                unsafe { TopicFilter::new_unchecked(MqttString::try_from("test/hello").unwrap()) },
                &SubscriptionOptions {
                    retain_handling: RetainHandling::AlwaysSend,
                    retain_as_published: false,
                    no_local: true,
                    qos: QoS::AtMostOnce,
                },
            ))
            .unwrap();

        topics
            .push(SubscriptionFilter::new(
                unsafe { TopicFilter::new_unchecked(MqttString::try_from("asdfjklo/#").unwrap()) },
                &SubscriptionOptions {
                    retain_handling: RetainHandling::NeverSend,
                    retain_as_published: true,
                    no_local: false,
                    qos: QoS::ExactlyOnce,
                },
            ))
            .unwrap();
        let packet: SubscribePacket<'_, 2> = SubscribePacket::new(23197, topics).unwrap();

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
        let mut topics = Vec::new();
        topics
            .push(SubscriptionFilter::new(
                unsafe { TopicFilter::new_unchecked(MqttString::try_from("abc/+/y").unwrap()) },
                &SubscriptionOptions {
                    retain_handling: RetainHandling::SendIfNotSubscribedBefore,
                    retain_as_published: true,
                    no_local: false,
                    qos: QoS::AtMostOnce,
                },
            ))
            .unwrap();

        let mut packet: SubscribePacket<'_, 10> = SubscribePacket::new(23197, topics).unwrap();
        packet
            .add_subscription_identifier(VarByteInt::try_from(87986078u32).unwrap())
            .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
                0x82, //
                0x12, // remaining length
                0x5A, // Packet identifier MSB
                0x9D, // Packet identifier LSB
                0x05, // Property length
                // Property - Subscription Identifier
                0x0B, 0x9E, 0x9F, 0xFA, 0x29, // Payload - Topic Filter
                0x00, 0x07, b'a', b'b', b'c', b'/', b'+', b'/', b'y', 0x18,
            ]
        );
    }
}
