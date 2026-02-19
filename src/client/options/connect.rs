use const_fn::const_fn;

use crate::{
    client::options::WillOptions,
    config::{KeepAlive, MaximumPacketSize, SessionExpiryInterval},
    types::{MqttBinary, MqttString},
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

    /// The maximum packet size that the client is willing to accept
    pub maximum_packet_size: MaximumPacketSize,

    /// When set to true, the broker may return response information used to construct response topics.
    pub request_response_information: bool,

    /// The user name the client wishes to authenticate with.
    pub user_name: Option<MqttString<'c>>,
    /// The password the client wishes to perform basic authentication with.
    pub password: Option<MqttBinary<'c>>,

    /// The will configuration for the session of the connection.
    pub will: Option<WillOptions<'c>>,
}

impl<'c> Default for Options<'c> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c> Options<'c> {
    /// Creates new connect options with no clean start, infinite keep alive and session
    /// expiry immediately after disconnecting.
    pub const fn new() -> Self {
        Self {
            clean_start: false,
            keep_alive: KeepAlive::Infinite,
            session_expiry_interval: SessionExpiryInterval::EndOnDisconnect,
            maximum_packet_size: MaximumPacketSize::Unlimited,
            request_response_information: false,
            user_name: None,
            password: None,
            will: None,
        }
    }

    /// Sets the clean start flag to true.
    pub const fn clean_start(mut self) -> Self {
        self.clean_start = true;
        self
    }
    /// Sets the desired keep alive of the connection.
    pub const fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = keep_alive;
        self
    }
    /// Sets the desired session expiry interval of the connection.
    pub const fn session_expiry_interval(mut self, interval: SessionExpiryInterval) -> Self {
        self.session_expiry_interval = interval;
        self
    }
    /// Sets the maximum packet size.
    pub const fn maximum_packet_size(mut self, maximum_packet_size: u32) -> Self {
        self.maximum_packet_size = MaximumPacketSize::Limit(maximum_packet_size);
        self
    }
    /// Sets the request response information property to true prompting the server to return
    /// a response information property to construct response topics.
    pub const fn request_response_information(mut self) -> Self {
        self.request_response_information = true;
        self
    }
    /// Sets the user name.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn user_name(mut self, user_name: MqttString<'c>) -> Self {
        self.user_name = Some(user_name);
        self
    }
    /// Sets the password.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn password(mut self, password: MqttBinary<'c>) -> Self {
        self.password = Some(password);
        self
    }
    /// Sets the will.
    #[const_fn(cfg(not(feature = "alloc")))]
    pub const fn will(mut self, will: WillOptions<'c>) -> Self {
        self.will = Some(will);
        self
    }
}
