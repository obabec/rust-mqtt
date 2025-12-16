use crate::config::SessionExpiryInterval;

/// Options for a disconnection to the server with a DISCONNECT packet.
pub struct Options {
    /// If set to true, the server publishes the will message.
    pub publish_will: bool,

    /// The session expiry interval property. Not allowed to be set to a non-zero value
    /// if the session expiry interval property in the CONNECT packet has been 0.
    pub session_expiry_interval: Option<SessionExpiryInterval>,
}
