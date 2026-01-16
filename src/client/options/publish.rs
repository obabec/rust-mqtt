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

    /// The topic that the message is published on. The topic can be referenced over
    /// an existing topic alias mapping or by specifying the topic name and optionally
    /// mapping a topic alias to it.
    pub topic: TopicReference<'p>,

    /// The quality of service that the message is published with to the server.
    /// The quality of service level used by the server to send this publication
    /// to subscribed clients is the minimum of this value and the quality of service
    /// value of the receiving client's subscription.
    pub qos: QoS,
}

/// The options for specifiying which topic to publish to. Topic aliases only last for the
/// duration of a single network connection and not necessarily until the session end.
///
/// Topic aliases must not be 0
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TopicReference<'t> {
    /// Publish to the inner topic name without creating an alias.
    Name(TopicName<'t>),

    /// Publish to an already mapped topic alias. The alias must have been defined earlier
    /// in the network connection.
    Alias(u16),

    /// Create a new topic alias or replace an existing topic alias.
    /// The alias lasts until the end of the network connection.
    Mapping(TopicName<'t>, u16),
}

impl<'t> TopicReference<'t> {
    pub(crate) fn alias(&self) -> Option<u16> {
        match self {
            Self::Name(_) => None,
            Self::Alias(alias) => Some(*alias),
            Self::Mapping(_, alias) => Some(*alias),
        }
    }
    pub(crate) fn topic_name(&self) -> Option<&TopicName<'t>> {
        match self {
            Self::Name(topic_name) => Some(topic_name),
            Self::Alias(_) => None,
            Self::Mapping(topic_name, _) => Some(topic_name),
        }
    }

    /// Delegates to `Bytes::as_borrowed()`.
    pub fn as_borrowed(&'t self) -> Self {
        match self {
            Self::Name(topic_name) => Self::Name(topic_name.as_borrowed()),
            Self::Alias(alias) => Self::Alias(*alias),
            Self::Mapping(topic_name, alias) => Self::Mapping(topic_name.as_borrowed(), *alias),
        }
    }
}
