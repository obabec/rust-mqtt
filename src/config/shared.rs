use crate::config::{KeepAlive, SessionExpiryInterval};

/// Negotiated configuration valid for the duration of a connection.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The maximum interval in seconds allowed to expire between sending two packets without the connection being closed.
    pub keep_alive: KeepAlive,

    /// The negotiated session expiry interval after the connection has been closed.
    pub session_expiry_interval: SessionExpiryInterval,
}
