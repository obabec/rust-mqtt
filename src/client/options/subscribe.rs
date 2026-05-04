use crate::types::{MqttStringPair, QoS, VarByteInt};

/// Options for subscription included for every topic.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'s> {
    /// Server-side retain handling configuration for this subscription.
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

    /// The maximum quality of service that the server is allowed to forward publications
    /// at matching this subscription with to the client. A quality of service level
    /// lower than this can be granted by the server.
    pub qos: QoS,

    /// The subscription identifier of the subscription. The server will set
    /// subscription identifier properties in its PUBLISH packets to the values of
    /// all matching subscriptions with a subscription identifier.
    ///
    /// Must be [`None`] when the server does not support retain (can be checked via
    /// [`Client::server_config`]). The client will not subscribe if a violation occurs
    /// but prevent the protocol error and return an error.
    ///
    /// [`Client::server_config`]: crate::client::Client::server_config
    pub subscription_identifier: Option<VarByteInt>,

    /// Arbitrary key-value pairs of strings sent as the user property entries of the
    /// SUBSCRIBE packet. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    pub user_properties: &'s [MqttStringPair<'s>],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'s> Options<'s> {
    /// Creates options with values coherent to the [`Default`] implementations of the fields and
    /// [`QoS::AtMostOnce`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            retain_handling: RetainHandling::AlwaysSend,
            retain_as_published: false,
            no_local: false,
            qos: QoS::AtMostOnce,
            subscription_identifier: None,
            user_properties: &[],
        }
    }

    /// Sets the Quality of Service level.
    #[must_use]
    pub const fn qos(mut self, qos: QoS) -> Self {
        self.qos = qos;
        self
    }
    /// Sets the Quality of Service level to 1 ([`QoS::AtLeastOnce`]).
    #[must_use]
    pub const fn at_least_once(self) -> Self {
        self.qos(QoS::AtLeastOnce)
    }
    /// Sets the Quality of Service level to 2 ([`QoS::ExactlyOnce`]).
    #[must_use]
    pub const fn exactly_once(self) -> Self {
        self.qos(QoS::ExactlyOnce)
    }
    /// Sets the server-side retain handling configuration for this subscription.
    #[must_use]
    pub const fn retain_handling(mut self, retain_handling: RetainHandling) -> Self {
        self.retain_handling = retain_handling;
        self
    }
    /// Sets the retain as published flag to true.
    #[must_use]
    pub const fn retain_as_published(mut self) -> Self {
        self.retain_as_published = true;
        self
    }
    /// Sets the no local flag to true.
    #[must_use]
    pub const fn no_local(mut self) -> Self {
        self.no_local = true;
        self
    }
    /// Sets the subscription identifier property.
    ///
    /// Note that this is only allowed if the server supports subscription identifiers.
    #[must_use]
    pub const fn subscription_identifier(mut self, subscription_identifier: VarByteInt) -> Self {
        self.subscription_identifier = Some(subscription_identifier);
        self
    }
    /// Sets the user properties. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    #[must_use]
    pub const fn user_properties(mut self, user_properties: &'s [MqttStringPair<'s>]) -> Self {
        self.user_properties = user_properties;
        self
    }
}

/// Server-side retain handling configuration for a subscription.
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
