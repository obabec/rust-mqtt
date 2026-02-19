use const_fn::const_fn;

use crate::{
    types::{MqttBinary, MqttString, QoS, TopicName, Will},
    v5::property::WillDelayInterval,
};

/// Options for configuring the client's will or last will in a session.
/// The server can publish a single PUBLISH packet in place of the client.
/// This process behaves as if the will message was published by the client.
/// The will is published at the earlier of the following scenarios:
/// - The session of the client ends.
/// - The will delay interval passes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'c> {
    /// The quality of service that the server publishes the will message with in place of the client.
    pub will_qos: QoS,

    /// The value of the retain flag of the will message published by the server in place of the client.
    pub will_retain: bool,

    /// The topic of the will publication.
    pub will_topic: TopicName<'c>,

    // Will properties starting here
    /// The interval in seconds that passes after a disconnection before the server publishes the will.
    /// The session of the client does not necessarily have to end for this scenario to happen.
    /// The client can reconnect before this interval has passed to prevent the will publication.
    /// If the value of the will delay interval is 0, the property is omitted on the network.
    pub will_delay_interval: u32,

    /// The payload format indicator property in the will publication. If present, this indicates
    /// whether the will payload is valid UTF-8. If set to [`None`], the property is omitted on the
    /// network.
    pub payload_format_indicator: Option<bool>,

    /// The message expiry interval in seconds of the will publication. If set to [`None`], the message
    /// does not expire and the message expiry interval property is omitted on the network.
    pub message_expiry_interval: Option<u32>,

    /// The content type property in the will publication. If set to [`None`], the property is omitted
    /// on the network.
    pub content_type: Option<MqttString<'c>>,

    /// The response topic property in the will publication. If set to [`None`], the property is omitted
    /// on the network.
    pub response_topic: Option<TopicName<'c>>,

    /// The correlation data property in the will publication. If set to [`None`], the property is omitted
    /// on the network.
    pub correlation_data: Option<MqttBinary<'c>>,

    /// The payload of the will publication.
    pub will_message: MqttBinary<'c>,
}

impl<'c> Options<'c> {
    /// Creates options with values coherent to the [`Default`] implementations of the fields and
    /// [`QoS::AtMostOnce`].
    pub const fn new(topic: TopicName<'c>, message: MqttBinary<'c>) -> Options<'c> {
        Options {
            will_qos: QoS::AtMostOnce,
            will_retain: false,
            will_topic: topic,
            will_delay_interval: 0,
            payload_format_indicator: None,
            message_expiry_interval: None,
            content_type: None,
            response_topic: None,
            correlation_data: None,
            will_message: message,
        }
    }

    /// Sets the Quality of Service level.
    pub const fn qos(mut self, qos: QoS) -> Self {
        self.will_qos = qos;
        self
    }
    /// Sets the Quality of Service level to 1 (At Least Once).
    pub const fn at_least_once(self) -> Self {
        self.qos(QoS::AtLeastOnce)
    }
    /// Sets the Quality of Service level to 1 (Exactly Once).
    pub const fn exactly_once(self) -> Self {
        self.qos(QoS::ExactlyOnce)
    }
    /// Sets the retain flag in the will message to true.
    pub const fn retain(mut self) -> Self {
        self.will_retain = true;
        self
    }
    /// Sets the delay in seconds after which the will message is published.
    pub const fn delay_interval(mut self, delay_interval: u32) -> Self {
        self.will_delay_interval = delay_interval;
        self
    }
    /// Sets the payload format indicator property to true thus marking the payload of the will message.
    /// as valid UTF-8.
    pub const fn payload_format_indicator(mut self, is_payload_utf8: bool) -> Self {
        self.payload_format_indicator = Some(is_payload_utf8);
        self
    }
    /// Sets the message expiry interval in seconds of the will message.
    pub const fn message_expiry_interval(mut self, message_expiry_interval: u32) -> Self {
        self.message_expiry_interval = Some(message_expiry_interval);
        self
    }
    /// Sets a custom content type of the will message.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn content_type(mut self, content_type: MqttString<'c>) -> Self {
        self.content_type = Some(content_type);
        self
    }
    /// Marks the will message as a request by setting the response topic property.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn response_topic(mut self, response_topic: TopicName<'c>) -> Self {
        self.response_topic = Some(response_topic);
        self
    }
    /// Sets the correlation data property in the request.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn correlation_data(mut self, correlation_data: MqttBinary<'c>) -> Self {
        self.correlation_data = Some(correlation_data);
        self
    }
}

impl<'c> Options<'c> {
    pub(crate) fn as_borrowed_will(&'c self) -> Will<'c> {
        Will {
            will_topic: self.will_topic.as_borrowed(),
            will_delay_interval: match self.will_delay_interval {
                0 => None,
                i => Some(WillDelayInterval(i)),
            },
            payload_format_indicator: self.payload_format_indicator.map(Into::into),
            message_expiry_interval: self.message_expiry_interval.map(Into::into),
            content_type: self
                .content_type
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            response_topic: self
                .response_topic
                .as_ref()
                .map(TopicName::as_borrowed)
                .map(Into::into),
            correlation_data: self
                .correlation_data
                .as_ref()
                .map(MqttBinary::as_borrowed)
                .map(Into::into),
            will_message: self.will_message.as_borrowed(),
        }
    }
}
