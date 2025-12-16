use heapless::Vec;

use crate::{
    eio::Write,
    header::{FixedHeader, PacketType},
    io::write::Writable,
    packet::{Packet, TxError, TxPacket},
    types::{MqttString, TooLargeToEncode, TopicFilter, VarByteInt},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UnsubscribePacket<'p, const MAX_TOPIC_FILTERS: usize> {
    packet_identifier: u16,
    topic_filters: Vec<TopicFilter<'p>, MAX_TOPIC_FILTERS>,
}

impl<'p, const MAX_TOPIC_FILTERS: usize> Packet for UnsubscribePacket<'p, MAX_TOPIC_FILTERS> {
    const PACKET_TYPE: PacketType = PacketType::Unsubscribe;
}
impl<'p, const MAX_TOPIC_FILTERS: usize> TxPacket for UnsubscribePacket<'p, MAX_TOPIC_FILTERS> {
    /// If MAX_TOPIC_FILTERS is to less than or equal to 4095, it is guaranteed that TxError::RemainingLenExceeded is never returned.
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x02, self.remaining_length()?)
            .write(write)
            .await?;

        self.packet_identifier.write(write).await?;
        self.properties_length().write(write).await?;

        for t in &self.topic_filters {
            t.as_ref().write(write).await?;
        }

        Ok(())
    }
}

impl<'p, const MAX_TOPIC_FILTERS: usize> UnsubscribePacket<'p, MAX_TOPIC_FILTERS> {
    fn remaining_length(&self) -> Result<VarByteInt, TooLargeToEncode> {
        let variable_header_length = self.packet_identifier.written_len();

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let body_length: usize = self
            .topic_filters
            .iter()
            .map(TopicFilter::as_ref)
            .map(MqttString::written_len)
            .sum();

        let total_length = variable_header_length + total_properties_length + body_length;

        // MAX_TOPIC_FILTERS has to be less than or equal to 4095 to guarantee:
        //   Max length = 3 + MAX_TOPIC_FILTERS * 65537 <= VarByteInt::MAX_ENCODABLE
        // packet identifier: 2
        // properties length: 1
        // properties: 0
        // topic filters: MAX_TOPIC_FILTERS * 65537
        VarByteInt::try_from(total_length as u32)
    }

    pub fn properties_length(&self) -> VarByteInt {
        // Safety: Max length = 0 < VarByteInt::MAX_ENCODABLE
        unsafe { VarByteInt::new_unchecked(0) }
    }
}

impl<'p, const MAX_TOPIC_FILTERS: usize> UnsubscribePacket<'p, MAX_TOPIC_FILTERS> {
    pub fn new(
        packet_identifier: u16,
        topic_filters: Vec<TopicFilter<'p>, MAX_TOPIC_FILTERS>,
    ) -> Self {
        Self {
            packet_identifier,
            topic_filters,
        }
    }
}

#[cfg(test)]
mod unit {
    use heapless::Vec;

    use crate::{
        test::tx::encode,
        types::{MqttString, TopicFilter},
        v5::packet::UnsubscribePacket,
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_payload() {
        let mut topics = Vec::new();

        topics
            .push(unsafe {
                TopicFilter::new_unchecked(MqttString::try_from("test/+/topic").unwrap())
            })
            .unwrap();
        topics
            .push(unsafe { TopicFilter::new_unchecked(MqttString::try_from("test/#").unwrap()) })
            .unwrap();

        let packet: UnsubscribePacket<'_, 2> = UnsubscribePacket::new(9874, topics);

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
}
