use crate::{
    config::{MaximumPacketSize, ReceiveMaximum},
    types::QoS,
};

/// Configuration of the server which must be upheld by the client.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// Maximum concurrent [`QoS`] 1 & 2 publications that the server is willing to accept.
    pub receive_maximum: ReceiveMaximum,

    /// Maximum supported [`QoS`] by the server.
    pub maximum_qos: QoS,

    /// Retained messages are supported by the server.
    pub retain_supported: bool,

    /// Maximum packet size that the server is willing to accept.
    pub maximum_packet_size: MaximumPacketSize,

    /// Highest value of a topic alias the server is willing to accept.
    /// Equal to the number of distinct topic aliases the server supports.
    pub topic_alias_maximum: u16,

    /// Serverside support for wildcard subscriptions.
    pub wildcard_subscription_supported: bool,
    /// Serverside support for subscription identifiers.
    pub subscription_identifiers_supported: bool,
    /// Serverside support for shared subscriptions.
    pub shared_subscription_supported: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            receive_maximum: ReceiveMaximum(u16::MAX),
            maximum_qos: QoS::ExactlyOnce,
            retain_supported: true,
            maximum_packet_size: Default::default(),
            topic_alias_maximum: 0,
            wildcard_subscription_supported: true,
            subscription_identifiers_supported: true,
            shared_subscription_supported: true,
        }
    }
}
