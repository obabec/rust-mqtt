use crate::{
    client::options::WillOptions,
    config::{KeepAlive, SessionExpiryInterval},
    types::{MqttBinary, MqttString},
};

/// Options for a connection.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'c> {
    /// If set to true, a new session is started. If set to false, an existing session is continued.
    pub clean_start: bool,

    /// The setting of the keep alive the client wishes. Can be set to a different value by the server.
    pub keep_alive: KeepAlive,

    /// The setting of the session expiry interval the server wishes. Can be set to a different value by the server.
    pub session_expiry_interval: SessionExpiryInterval,

    /// When set to true, the broker may return response information used to construct response topics.
    pub request_response_information: bool,

    /// The user name the client wishes to authenticate with.
    pub user_name: Option<MqttString<'c>>,
    /// The user name the client wishes to perform basic authentication with.
    pub password: Option<MqttBinary<'c>>,

    /// The will configuration for the session of the connection.
    pub will: Option<WillOptions<'c>>,
}

impl<'c> Options<'c> {
    /// Creates new connect options with no clean start, infinite keep alive and no session expiry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the clean start flag to true.
    pub fn clean_start(mut self) -> Self {
        self.clean_start = true;
        self
    }
    /// Sets the desired keep alive of the connection.
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = keep_alive;
        self
    }
    /// Sets the desired session expiry interval of the connection.
    pub fn session_expiry_interval(mut self, interval: SessionExpiryInterval) -> Self {
        self.session_expiry_interval = interval;
        self
    }
    /// Sets the request response information property to true prompting the server to return
    /// a response information property to construct construct response topics.
    pub fn request_response_information(mut self) -> Self {
        self.request_response_information = true;
        self
    }
    /// Sets the user name.
    pub fn user_name(mut self, user_name: MqttString<'c>) -> Self {
        self.user_name = Some(user_name);
        self
    }
    /// Sets the password.
    pub fn password(mut self, password: MqttBinary<'c>) -> Self {
        self.password = Some(password);
        self
    }
    /// Sets the will.
    pub fn will(mut self, will: WillOptions<'c>) -> Self {
        self.will = Some(will);
        self
    }
}
