use crate::types::PacketIdentifier;

/// MQTT's Quality of Service
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum QoS {
    /// Quality of Service Level 0. Publications with this level are only sent once.
    AtMostOnce = 0,
    /// Quality of Service Level 1. Publications with this level are sent until a PUBACK indicates that it was received.
    AtLeastOnce = 1,
    /// Quality of Service Level 2. Publications with this level are followed by a handshake assuring it is received once.
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
    pub const fn packet_identifier(&self) -> Option<PacketIdentifier> {
        match self {
            Self::AtMostOnce => None,
            Self::AtLeastOnce(pid) => Some(*pid),
            Self::ExactlyOnce(pid) => Some(*pid),
        }
    }
}
