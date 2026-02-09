use crate::config::SessionExpiryInterval;

/// Options for a disconnection to the server with a DISCONNECT packet.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options {
    /// If set to true, the server publishes the will message.
    pub publish_will: bool,

    /// The session expiry interval property. Not allowed to be set to a non-zero value
    /// if the session expiry interval property in the CONNECT packet has been 0.
    /// This value overrides the session expiry interval negotiated in the handshake.
    pub session_expiry_interval: Option<SessionExpiryInterval>,
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    /// Creates new disconnect options with will publication disabled and no session expiry interval.
    pub const fn new() -> Self {
        Self {
            publish_will: false,
            session_expiry_interval: None,
        }
    }

    /// Sets the publish will flag to true.
    pub const fn publish_will(mut self) -> Self {
        self.publish_will = true;
        self
    }
    /// Sets the session expiry interval property.
    pub const fn session_expiry_interval(mut self, interval: SessionExpiryInterval) -> Self {
        self.session_expiry_interval = Some(interval);
        self
    }
}
