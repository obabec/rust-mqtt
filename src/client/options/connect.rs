use crate::{
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString, QoS, Will},
    v5::property::{PayloadFormatIndicator, WillDelayInterval},
};

/// Options for a connection.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'c> {
    /// If set to true, a new session is started. If set to false, an existing session is continued.
    pub clean_start: bool,

    /// The setting of the keep alive the client wishes. Can be set to a different value by the server.
    pub keep_alive: KeepAlive,

    /// The setting of the session expiry interval the server wishes. Can be set to a different value by the server.
    pub session_expiry_interval: SessionExpiryInterval,

    /// The user name the client wishes to authenticate with.
    pub user_name: Option<MqttString<'c>>,
    /// The user name the client wishes to perform basic authentication with.
    pub password: Option<MqttBinary<'c>>,

    /// The will configuration for the session of the connection.
    pub will: Option<WillOptions<'c>>,
}

/// Options for configuring the client's will or last will in a session.
/// The server can publish a single PUBLISH packet in place of the client.
/// This process behaves as if the will message was published by the client.
/// The will is published at the earlier of the following scenarios:
/// - The session of the client ends.
/// - The will delay interval passes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WillOptions<'c> {
    /// The quality of service that the server publishes the will message with in place of the client.
    pub will_qos: QoS,

    /// The value of the retain flag of the will message published by the server in place of the client.
    pub will_retain: bool,

    /// The topic of the will publication.
    pub will_topic: MqttString<'c>,

    /// The payload of the will publication.
    pub will_payload: MqttBinary<'c>,

    // Will properties starting here
    /// The interval in seconds that passes after a disconnection before the server publishes the will.
    /// The session of the client does not necessarily have to end for this scenario to happen.
    /// The client can reconnect before this interval has passed to prevent the will publication.
    /// If the value of the will delay interval is 0, the property is omitted on the network.
    pub will_delay_interval: u32,

    /// The payload format indicator property in the will publication. If set to false, the property
    /// is omitted on the network.
    pub is_payload_utf8: bool,

    /// The message expiry interval in seconds of the will publication. If set to `None`, the message
    /// does not expire and the message expiry interval property is omitted on the network.
    pub message_expiry_interval: Option<u32>,

    /// The content type property in the will publication. If set to `None`, the property is omitted
    /// on the network.
    pub content_type: Option<MqttString<'c>>,

    /// The response topic property in the will publication. If set to `None`, the property is omitted
    /// on the network.
    pub response_topic: Option<MqttString<'c>>,

    /// The correlation data property in the will publication. If set to `None`, the property is omitted
    /// on the network.
    pub correlation_data: Option<MqttBinary<'c>>,
}

impl<'c> WillOptions<'c> {
    pub(crate) fn as_will(&'c self) -> Will<'c> {
        Will {
            will_topic: self.will_topic.as_borrowed(),
            will_delay_interval: match self.will_delay_interval {
                0 => None,
                i => Some(WillDelayInterval(i)),
            },
            payload_format_indicator: match self.is_payload_utf8 {
                false => None,
                true => Some(PayloadFormatIndicator(true)),
            },
            message_expiry_interval: self.message_expiry_interval.map(Into::into),
            content_type: self
                .content_type
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            response_topic: self
                .response_topic
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            correlation_data: self
                .correlation_data
                .as_ref()
                .map(MqttBinary::as_borrowed)
                .map(Into::into),
            will_payload: self.will_payload.as_borrowed(),
        }
    }
}
