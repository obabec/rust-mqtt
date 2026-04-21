use core::num::NonZero;

use const_fn::const_fn;

use crate::{
    client::options::WillOptions,
    config::{KeepAlive, MaximumPacketSize, SessionExpiryInterval},
    types::{MqttBinary, MqttString, MqttStringPair},
};

/// Clientside options for establishing a connection to a broker.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'c> {
    /// If set to true, a new session is started. If set to false and the server has an existing session,
    /// it is continued.
    pub clean_start: bool,

    /// The setting of the keep alive the client wishes. Can be set to a different value by the server.
    pub keep_alive: KeepAlive,

    /// The setting of the session expiry interval the client wishes. This setting can be overriden by
    /// the server. If set to [`SessionExpiryInterval::EndOnDisconnect`], the session expiry interval
    /// property is omitted on the network.
    pub session_expiry_interval: SessionExpiryInterval,

    /// The maximum packet size that the client is willing to accept. If set to
    /// [`MaximumPacketSize::Unlimited`], the maximum packet size property is omitted on the network.
    pub maximum_packet_size: MaximumPacketSize,

    /// When set to true, the server may return response information used to construct response topics.
    /// If set to false, the server must not return response information and the request response
    /// information property is omitted on the network.
    pub request_response_information: bool,

    /// When set to false, the server must not send a reason string or user properties on any packet
    /// other than PUBLISH, CONNACK, or DISCONNECT.
    /// If set to true, the server is allowed to send reason strings and user properties on any packet
    /// that specifies these properties and the request problem information property is omitted on the
    /// network.
    pub request_problem_information: bool,

    /// Arbitrary key-value pairs of strings sent as the user property entries of the CONNECT packet.
    /// Note that this slice's length must be less than [`Client`]'s const generic parameter
    /// `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    pub user_properties: &'c [MqttStringPair<'c>],

    /// The user name the client wishes to authenticate with.
    pub user_name: Option<MqttString<'c>>,
    /// The password the client wishes to perform basic authentication with.
    pub password: Option<MqttBinary<'c>>,

    /// The will configuration for the session of the connection.
    pub will: Option<WillOptions<'c>>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c> Options<'c> {
    /// Creates new connect options with no clean start, infinite keep alive and session
    /// expiry immediately after disconnecting. Request problem information is false
    /// disallowing the server from sending user properties or reason strings in PUBACK,
    /// PUBREC, PUBREL, PUBCOMP, SUBACK and UNSUBACK packets.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            clean_start: false,
            keep_alive: KeepAlive::Infinite,
            session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
            maximum_packet_size: MaximumPacketSize::Unlimited,
            request_response_information: false,
            request_problem_information: false,
            user_properties: &[],
            user_name: None,
            password: None,
            will: None,
        }
    }

    /// Sets the clean start flag to true.
    #[must_use]
    pub const fn clean_start(mut self) -> Self {
        self.clean_start = true;
        self
    }
    /// Sets the desired keep alive of the connection.
    #[must_use]
    pub const fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = keep_alive;
        self
    }
    /// Sets the desired session expiry interval of the connection.
    #[must_use]
    pub const fn session_expiry_interval(mut self, interval: SessionExpiryInterval) -> Self {
        self.session_expiry_interval = interval;
        self
    }
    /// Sets the maximum packet size as a limit in bytes. A value less than 2 does not make sense.
    #[must_use]
    pub const fn maximum_packet_size(mut self, maximum_packet_size: NonZero<u32>) -> Self {
        self.maximum_packet_size = MaximumPacketSize::Limit(maximum_packet_size);
        self
    }
    /// Sets the request response information property to true prompting the server to return
    /// a response information property to construct response topics.
    #[must_use]
    pub const fn request_response_information(mut self) -> Self {
        self.request_response_information = true;
        self
    }
    /// Sets the request problem information property to true allowing the server to send
    /// user properties and reason strings on all packets instead of only PUBLISH,
    /// CONNACK and DISCONNECT.
    #[must_use]
    pub const fn request_problem_information(mut self) -> Self {
        self.request_problem_information = true;
        self
    }
    /// Sets the user properties. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    #[must_use]
    pub const fn user_properties(mut self, user_properties: &'c [MqttStringPair<'c>]) -> Self {
        self.user_properties = user_properties;
        self
    }
    /// Sets the user name.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub const fn user_name(mut self, user_name: MqttString<'c>) -> Self {
        self.user_name = Some(user_name);
        self
    }
    /// Sets the password.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub const fn password(mut self, password: MqttBinary<'c>) -> Self {
        self.password = Some(password);
        self
    }
    /// Sets the will.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub const fn will(mut self, will: WillOptions<'c>) -> Self {
        self.will = Some(will);
        self
    }
}
