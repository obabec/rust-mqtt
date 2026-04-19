use crate::{config::SessionExpiryInterval, types::MqttStringPair};

/// Options for a disconnection to the server with a DISCONNECT packet.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'d> {
    /// If set to true, the client uses [`ReasonCode::DisconnectWithWillMessage`] in the
    /// DISCONNECT packet and the server publishes the will message.
    ///
    /// [`ReasonCode::DisconnectWithWillMessage`]: crate::types::ReasonCode::DisconnectWithWillMessage
    pub publish_will: bool,

    /// The session expiry interval property. Not allowed to be set to a non-zero value
    /// if the session expiry interval property in the CONNECT packet has been 0.
    /// This value overrides the session expiry interval negotiated in the handshake.
    pub session_expiry_interval: Option<SessionExpiryInterval>,

    /// Arbitrary key-value pairs of strings sent as the user property entries of the DISCONNECT
    /// packet. Note that this slice's length must be less than [`crate::client::Client`]'s const
    /// generic parameter `MAX_USER_PROPERTIES`.
    pub user_properties: &'d [MqttStringPair<'d>],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d> Options<'d> {
    /// Creates new disconnect options with will publication disabled and no session expiry interval.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            publish_will: false,
            session_expiry_interval: None,
            user_properties: &[],
        }
    }

    /// Sets the publish will flag to true.
    #[must_use]
    pub const fn publish_will(mut self) -> Self {
        self.publish_will = true;
        self
    }
    /// Sets the session expiry interval property.
    #[must_use]
    pub const fn session_expiry_interval(mut self, interval: SessionExpiryInterval) -> Self {
        self.session_expiry_interval = Some(interval);
        self
    }
    /// Sets the user properties. Note that this slice's length must be less than
    /// [`crate::client::Client`]'s const generic parameter `MAX_USER_PROPERTIES`.
    #[must_use]
    pub const fn user_properties(mut self, user_properties: &'d [MqttStringPair<'d>]) -> Self {
        self.user_properties = user_properties;
        self
    }
}
