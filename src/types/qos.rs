use crate::types::PacketIdentifier;

/// MQTT's Quality of Service levels
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum QoS {
    /// Quality of Service Level 0. Reliable delivery of publications at this level is guaranteed solely
    /// by the underlying transport mechanism within a continuous network connection. Publications can
    /// be lost if a disconnection on protocol or transport level occurs around the transmission time.
    /// A single message is never delivered more than once. Retransmissions cannot be marked as such on
    /// protocol level.
    AtMostOnce = 0,

    /// Quality of Service Level 1. Reliable delivery of publications is guaranteed at the cost of a
    /// two-way handshake on protocol level, even with disconnects around (re-)transmission time,
    /// given a reconnection to the same, non-expired session is successful and a retransmission is
    /// subsequently attempted. A single message can be delivered more than once, but actual duplicates
    /// cannot reliably be distinguished from needed retransmissions.
    AtLeastOnce = 1,

    /// Quality of Service Level 2. Reliable, exactly-once delivery of publications is guaranteed at
    /// the cost of a four-way handshake at protocol level, even with disconnects around
    /// (re-)transmission time, given a reconnection to the same, non-expired session is successful and
    /// retransmission of the PUBLISH or PUBREL packets is subsequently attempted. Delivery of
    /// publications may be further delayed than at other quality of service levels due to the required
    /// release of the message after the three-way handshake within the four-way handshake.
    ExactlyOnce = 2,
}
impl From<IdentifiedQoS> for QoS {
    fn from(value: IdentifiedQoS) -> Self {
        match value {
            IdentifiedQoS::AtMostOnce => Self::AtMostOnce,
            IdentifiedQoS::AtLeastOnce(_) => Self::AtLeastOnce,
            IdentifiedQoS::ExactlyOnce(_) => Self::ExactlyOnce,
        }
    }
}

impl QoS {
    pub(crate) const fn into_bits(self, left_shift: u8) -> u8 {
        let bits = match self {
            Self::AtMostOnce => 0x00,
            Self::AtLeastOnce => 0x01,
            Self::ExactlyOnce => 0x02,
        };

        bits << left_shift
    }

    pub(crate) const fn try_from_bits(bits: u8) -> Option<Self> {
        match bits {
            0x00 => Some(Self::AtMostOnce),
            0x01 => Some(Self::AtLeastOnce),
            0x02 => Some(Self::ExactlyOnce),
            _ => None,
        }
    }
}

/// MQTT's Quality of Service with special reference to the PUBLISH packet which contains
/// a packet identifier if its Quality of Service is greater than 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum IdentifiedQoS {
    /// Quality of Service Level 0. PUBLISH packets do not contain a packet identifier.
    AtMostOnce,
    /// Quality of Service Level 1. PUBLISH packets contain the included packet identifier.
    AtLeastOnce(PacketIdentifier),
    /// Quality of Service Level 2. PUBLISH packets contain the included packet identifier.
    ExactlyOnce(PacketIdentifier),
}

impl IdentifiedQoS {
    /// Returns [`Some`] if [`QoS`] > 0 and therefore has to be identified, and
    /// [`None`] otherwise.
    #[inline]
    #[must_use]
    pub const fn packet_identifier(&self) -> Option<PacketIdentifier> {
        match self {
            Self::AtMostOnce => None,
            Self::AtLeastOnce(pid) | Self::ExactlyOnce(pid) => Some(*pid),
        }
    }
}
