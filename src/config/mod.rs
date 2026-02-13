//! Contains configuration primitives and accumulations for server and client configuration.

mod client;
mod server;
mod shared;

pub use client::Config as ClientConfig;
pub use server::Config as ServerConfig;
pub use shared::Config as SharedConfig;

/// Keep alive mechanism within a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum KeepAlive {
    /// There is no keep alive mechanism. Any amount of time can pass between 2 MQTT packets
    #[default]
    Infinite,

    /// The maximum time interval in seconds allowed to pass between 2 MQTT packets.
    ///
    /// Must be greater than 0.
    Seconds(u16),
}

impl KeepAlive {
    pub(crate) fn as_u16(&self) -> u16 {
        match self {
            KeepAlive::Infinite => u16::MAX,
            KeepAlive::Seconds(s) => *s,
        }
    }
}

/// The handling of a session after a disconnection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SessionExpiryInterval {
    /// The session ends the moment a DISCONNECT packet is sent or the network connection closes.
    #[default]
    EndOnDisconnect,
    /// The session is not ended under any circumstances.
    NeverEnd,
    /// The session ends after this many seconds have passed after a DISCONNECT packet is sent or the network connection closes.
    Seconds(u32),
}

/// Maximum packet size. Exceeding this is a protocol error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum MaximumPacketSize {
    /// There is no imposed limit on how large packets can be.
    /// The technical limit is [`crate::types::VarByteInt::MAX_ENCODABLE`] + 5 (size of fixed header).
    #[default]
    Unlimited,

    /// There is a limit on how large packets can be.
    Limit(u32),
}

/// Maximum concurrent publications with a Quality of Service > 0.
///
/// Default is 65536 / [`u16::MAX`] and is used when no receive maximum is present. Can't be zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ReceiveMaximum(pub(crate) u16);
