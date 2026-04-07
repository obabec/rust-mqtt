use heapless::Vec;

use crate::{
    eio::Write,
    header::{FixedHeader, PacketType},
    io::write::Writable,
    packet::{Packet, TxError, TxPacket},
    types::{PacketIdentifier, TooLargeToEncode, TopicFilter, VarByteInt},
    v5::property::UserProperty,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnsubscribePacket<'p, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> {
    packet_identifier: PacketIdentifier,

    user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,

    topic_filters: Vec<TopicFilter<'p>, MAX_TOPIC_FILTERS>,
}

impl<const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> Packet
    for UnsubscribePacket<'_, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    const PACKET_TYPE: PacketType = PacketType::Unsubscribe;
}
impl<const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> TxPacket
    for UnsubscribePacket<'_, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    fn remaining_len(&self) -> VarByteInt {
        // Safety: UNSUBSCRIBE packets that are too long to encode cannot be created
        unsafe { self.remaining_len_raw().unwrap_unchecked() }
    }

    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x02, self.remaining_len())
            .write(write)
            .await?;

        self.packet_identifier.write(write).await?;
        self.properties_length().write(write).await?;

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        for t in &self.topic_filters {
            t.write(write).await?;
        }

        Ok(())
    }
}

impl<'p, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize>
    UnsubscribePacket<'p, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    /// If `MAX_TOPIC_FILTERS` is to less than or equal to 2053 and `MAX_USER_PROPERTIES` is
    /// less than or equal to 1021, it is guaranteed that `TooLargeToEncode` is never returned.
    pub fn new(
        packet_identifier: PacketIdentifier,
        user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
        topic_filters: Vec<TopicFilter<'p>, MAX_TOPIC_FILTERS>,
    ) -> Result<Self, TooLargeToEncode> {
        let p = Self {
            packet_identifier,
            user_properties,
            topic_filters,
        };

        // Reference to `UnsubscribePacket::remaining_len_raw` as to why this is true.
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

        let body_length: usize = self.topic_filters.iter().map(Writable::written_len).sum();

        let total_length = variable_header_length + total_properties_length + body_length;

        // max length = MAX_USER_PROPERTIES * 131077 + MAX_TOPIC_FILTERS * 65537 + 6
        // Invariant: The following inequation must be fulfilled to guarantee
        // max length <= VarByteInt::MAX_ENCODABLE:
        //
        // MAX_USER_PROPERTIES * 131077 + MAX_TOPIC_FILTERS * 65537 + 6 <= VarByteInt::MAX_ENCODABLE
        //
        // Given the maximum allowed value of MAX_USER_PROPERTIES = 1021 in the client, the following
        // inequation must be fulfilled:
        //
        // 1021 * 131077 + MAX_TOPIC_FILTERS * 65537 + 6 <= VarByteInt::MAX_ENCODABLE
        //
        // This results in the statement:
        //
        // MAX_TOPIC_FILTERS <= 2053 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // packet identifier: 2
        // property length: 4
        // properties: MAX_USER_PROPERTIES * 131077
        // topic filters: MAX_TOPIC_FILTERS * 65537
        VarByteInt::try_from(total_length as u32)
    }

    pub fn properties_length(&self) -> VarByteInt {
        let len = self
            .user_properties
            .iter()
            .map(Writable::written_len)
            .sum::<usize>();

        // max length = MAX_USER_PROPERTIES * 131077
        // Invariant: MAX_USER_PROPERTIES <= 2047 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // user properties: MAX_USER_PROPERTIES * 131077
        VarByteInt::new_unchecked(len as u32)
    }
}

#[cfg(test)]
mod unit {
    use core::num::NonZero;

    use heapless::Vec;

    use crate::{
        test::tx::encode,
        types::{MqttString, MqttStringPair, PacketIdentifier, TopicFilter},
        v5::{packet::UnsubscribePacket, property::UserProperty},
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_payload() {
        let topics = [
            TopicFilter::new(MqttString::try_from("test/+/topic").unwrap()).unwrap(),
            TopicFilter::new(MqttString::try_from("test/#").unwrap()).unwrap(),
        ];

        let packet = UnsubscribePacket::<2, 16>::new(
            PacketIdentifier::new(NonZero::new(9874).unwrap()),
            Vec::new(),
            topics.into(),
        )
        .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
                0xA2,
                0x19,
                0x26, // Packet identifier MSB
                0x92, // Packet identifier LSB
                0x00, // Property length
                // Payload
                0x00, 0x0C, b't', b'e', b's', b't', b'/', b'+', b'/', b't', b'o', b'p', b'i', b'c',
                // Payload
                0x00, 0x06, b't', b'e', b's', b't', b'/', b'#',
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_properties() {
        let topics = [TopicFilter::new(
            MqttString::try_from("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
        )
        .unwrap()];
        let user_properties = [
            UserProperty(MqttStringPair::new(
                MqttString::from_str("give up").unwrap(),
                MqttString::from_str("false").unwrap(),
            )),
            UserProperty(MqttStringPair::new(
                MqttString::from_str("let down").unwrap(),
                MqttString::from_str("false").unwrap(),
            )),
            UserProperty(MqttStringPair::new(
                MqttString::from_str("run around").unwrap(),
                MqttString::from_str("false").unwrap(),
            )),
            UserProperty(MqttStringPair::new(
                MqttString::from_str("hurt").unwrap(),
                MqttString::from_str("false").unwrap(),
            )),
        ];

        let packet = UnsubscribePacket::<1, 16>::new(
            PacketIdentifier::new(NonZero::new(23913).unwrap()),
            user_properties.into(),
            topics.into(),
        )
        .unwrap();

        #[rustfmt::skip]
        encode!(packet, [
                0xA2,
                0x75,
                0x5D, // Packet identifier MSB
                0x69, // Packet identifier LSB
                0x45, // Property length

                // User Property
                0x26,
                0x00, 0x07, b'g', b'i', b'v', b'e', b' ', b'u', b'p',
                0x00, 0x05, b'f', b'a', b'l', b's', b'e',

                // User Property
                0x26,
                0x00, 0x08, b'l', b'e', b't', b' ', b'd', b'o', b'w', b'n',
                0x00, 0x05, b'f', b'a', b'l', b's', b'e',

                // User Property
                0x26,
                0x00, 0x0A, b'r', b'u', b'n', b' ', b'a', b'r', b'o', b'u', b'n', b'd',
                0x00, 0x05, b'f', b'a', b'l', b's', b'e',

                // User Property
                0x26,
                0x00, 0x04, b'h', b'u', b'r', b't',
                0x00, 0x05, b'f', b'a', b'l', b's', b'e',

                // Payload
                0x00, 0x2B, b'h', b't', b't', b'p', b's', b':', b'/', b'/', b'w', b'w', b'w', b'.', b'y',
                b'o', b'u', b't', b'u', b'b', b'e', b'.', b'c', b'o', b'm', b'/', b'w', b'a', b't', b'c',
                b'h', b'?', b'v', b'=', b'd', b'Q', b'w', b'4', b'w', b'9', b'W', b'g', b'X', b'c', b'Q', 
            ]
        );
    }
}
