use crate::config::{KeepAlive, SessionExpiryInterval};

/// Negotiated configuration valid for the duration of a connection.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The negotiated [`KeepAlive`] interval in seconds allowed to expire between
    /// sending two packets without the connection being closed.
    pub keep_alive: KeepAlive,

    /// The negotiated [`SessionExpiryInterval`] after the connection has been closed.
    /// This value may be altered by the client with a DISCONNECT packet, but if the
    /// session expiry interval requested with the CONNECT packet (in [`ClientConfig`])
    /// is [`SessionExpiryInterval::EndOnDisconnect`] (a value of zero), a non-zero
    /// session expiry interval value in the DISCONNECT packet is not allowed.
    ///
    /// [`ClientConfig`]: crate::config::ClientConfig
    pub session_expiry_interval: SessionExpiryInterval,
}
