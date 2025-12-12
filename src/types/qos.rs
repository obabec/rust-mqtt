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

impl QoS {
    pub(crate) fn into_bits(self, left_shift: u8) -> u8 {
        let bits = match self {
            Self::AtMostOnce => 0x00,
            Self::AtLeastOnce => 0x01,
            Self::ExactlyOnce => 0x02,
        };

        bits << left_shift
    }

    pub(crate) fn try_from_bits(bits: u8) -> Result<Self, ()> {
        match bits {
            0x00 => Ok(Self::AtMostOnce),
            0x01 => Ok(Self::AtLeastOnce),
            0x02 => Ok(Self::ExactlyOnce),
            _ => Err(()),
        }
    }
}
