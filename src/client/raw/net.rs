use core::{matches, mem};

use crate::{fmt::debug_assert, io::Transport, types::ReasonCode};

/// Represents the state of a network connection, including mechanisms for handling failures gracefully.
#[derive(Debug, Default)]
pub(crate) enum NetState<N: Transport> {
    /// Both the transport layer and the MQTT protocol-level connection are healthy.
    Ok(N),

    /// The transport layer is healthy, but a protocol error (e.g. `MalformedPacket`) has occurred. A
    /// `DISCONNECT` packet must be sent with the provided [`ReasonCode`] before closing the connection.
    DueDisconnect(N, ReasonCode),

    /// The connection is inactive. This occurs if a protocol failure does not require a `DISCONNECT`,
    /// or if the transport itself encountered an error. The transport is retained so the user can close
    /// it gracefully or reuse it.
    Inactive(N),

    /// No network connection is available.
    #[default]
    Terminated,
}

pub enum Error {
    Faulted,
    Inactive,
    Terminated,
}

impl<N: Transport> NetState<N> {
    /// Returns `true` if the net state is [`Ok`].
    ///
    /// [`Ok`]: NetState::Ok
    #[must_use]
    pub(crate) fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }
    /// Returns `true` if the net state is [`Terminated`].
    ///
    /// [`Terminated`]: NetState::Terminated
    #[must_use]
    pub(crate) fn is_terminated(&self) -> bool {
        matches!(self, Self::Terminated)
    }

    pub fn replace(&mut self, net: N) {
        debug_assert!(
            self.is_terminated(),
            "network must be in Terminated state to replace it",
        );

        *self = Self::Ok(net);
    }
    pub fn get(&mut self) -> Result<&mut N, Error> {
        match self {
            Self::Ok(n) => Ok(n),
            Self::DueDisconnect(_, _) => Err(Error::Faulted),
            Self::Inactive(_) => Err(Error::Inactive),
            Self::Terminated => Err(Error::Terminated),
        }
    }

    pub fn fail(&mut self, reason_code: ReasonCode) {
        debug_assert!(
            matches!(self, Self::Ok(_)),
            "network must be in Ok(N) state to fail."
        );

        *self = match mem::take(self) {
            Self::Ok(n) | Self::DueDisconnect(n, _) => Self::DueDisconnect(n, reason_code),
            Self::Inactive(n) => Self::Inactive(n),
            Self::Terminated => Self::Terminated,
        }
    }

    pub fn deactivate(&mut self) {
        debug_assert!(
            matches!(self, Self::Ok(_) | Self::DueDisconnect(_, _)),
            "network must be in Ok(N) or DueDisconnect(N, ReasonCode) state to be deactivated."
        );

        *self = match mem::take(self) {
            Self::Ok(n) | Self::DueDisconnect(n, _) | Self::Inactive(n) => Self::Inactive(n),
            Self::Terminated => Self::Terminated,
        }
    }

    pub fn terminate(&mut self) -> Self {
        mem::take(self)
    }
}
