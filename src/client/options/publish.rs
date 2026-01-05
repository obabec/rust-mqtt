use crate::types::{QoS, TopicName};

/// Options for a publication.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'p> {
    /// Depicts the value of the retain flag in the PUBLISH packet.
    /// If set to 1, the server should retain the message on this topic.
    /// Retained messages with quality of service 0 can be discarded
    /// at any time by the server.
    pub retain: bool,

    /// The message expiry interval in seconds of this application message. After this
    /// interval has passed, the server cannot publish this message onward to subscribers.
    /// If set to `None`, the message does not expire and the message expiry interval
    /// property is omitted on the network.
    pub message_expiry_interval: Option<u32>,

    /// The topic that the message is published on.
    pub topic: TopicName<'p>,

    /// The quality of service that the message is published with to the server.
    /// The quality of service level used by the server to send this publication
    /// to subscribed clients is the minimum of this value and the quality of service
    /// value of the receiving client's subscription.
    pub qos: QoS,
}
