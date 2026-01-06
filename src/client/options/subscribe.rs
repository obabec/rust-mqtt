use crate::types::{QoS, VarByteInt};

/// Options for subscription included for every topic.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options {
    /// Serverside retain handling configuration for this subscription.
    pub retain_handling: RetainHandling,

    /// If set to true, the server sets the retain flag of a PUBLISH packet matching
    /// this subscription to the retain flag value of the original publication.
    /// If set to false, the server sets the retain flag of a PUBLISH packet matching
    /// this subscription to false. This does not apply for retained messages sent
    /// directly after subscribing - these messages always have the retain flag set to 1.
    pub retain_as_published: bool,

    /// If set to true, the server does not forward any publications matching
    /// this subscription to connections with client identifiers the same as
    /// the client identifier of this connection.
    /// If set to true on a shared subscription, a protocol error is triggered.
    pub no_local: bool,

    /// The maximum quality of server that the server can forward publications
    /// matching this subscription with to the client. A quality of service level
    /// lower than this can be granted by the server.
    pub qos: QoS,

    /// The subscription identifier of the subscription. The server will set
    /// subscription identifier properties in its PUBLISH packets to the values of
    /// all matching subscriptions with a subscription identifier.
    pub subscription_identifier: Option<VarByteInt>,
}

/// Serverside retain handling configuration for a subscription.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RetainHandling {
    /// Retained messages are always sent at the time of the subscribe.
    #[default]
    AlwaysSend,
    /// Retained messages are only sent if the subscription did not exist before.
    SendIfNotSubscribedBefore,
    /// Retained messages are never sent.
    NeverSend,
}
