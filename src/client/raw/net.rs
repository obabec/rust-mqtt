use core::mem;

use crate::{io::Transport, types::ReasonCode};

/// Represents a network connection with different variants for handling failures in the connection gracefully.
#[derive(Debug, Default)]
pub(crate) enum NetState<N: Transport> {
    /// The connection and dialogue with the server is ok
    Ok(N),

    /// The connection is ok but some protocol specific error (e.g. MalformedPacket or ProtocolError occured)
    ///
    /// Sending messages can be considered, but the session should ultimately be closed according to spec
    Faulted(N, ReasonCode),

    /// The connection is closed
    #[default]
    Terminated,
}

/// A network connection state related error.
pub enum Error {
    /// An error has occurred in the application MQTT communication.
    /// The underlying network is still open and should be used for sending a DISCONNECT packet.
    Faulted,

    /// An error has occurred in the application MQTT communication.
    /// The underlying network has already been closed because either a DISCONNECT packet has already been sent or shouldn't be sent at all.
    Terminated,
}

impl<N: Transport> NetState<N> {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
    pub fn replace(&mut self, net: N) {
        *self = Self::Ok(net);
    }
    pub fn get(&mut self) -> Result<&mut N, Error> {
        match self {
            Self::Ok(n) => Ok(n),
            Self::Faulted(_, _) => Err(Error::Faulted),
            Self::Terminated => Err(Error::Terminated),
        }
    }
    pub fn get_reason_code(&mut self) -> Option<ReasonCode> {
        match self {
            Self::Ok(_) => None,
            Self::Faulted(_, r) => Some(*r),
            Self::Terminated => None,
        }
    }

    pub fn fail(&mut self, reason_code: ReasonCode) {
        *self = match mem::take(self) {
            Self::Ok(n) | Self::Faulted(n, _) => Self::Faulted(n, reason_code),
            Self::Terminated => Self::Terminated,
        }
    }

    pub fn terminate(&mut self) -> Option<N> {
        match mem::take(self) {
            Self::Ok(n) => Some(n),
            Self::Faulted(n, _) => Some(n),
            Self::Terminated => None,
        }
    }
}
