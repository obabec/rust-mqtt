use core::matches;

use const_fn::const_fn;

use crate::types::{MqttString, MqttStringPair};

/// Options for an acknowledgement to the server with a PUBACK, PUBREC, PUBREL or PUBCOMP packet.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'a> {
    /// The reason string property of the PUBACK, PUBREC, PUBREL or PUBCOMP packet.
    pub reason_string: Option<MqttString<'a>>,

    /// Arbitrary key-value pairs of strings sent as the user property entries of the PUBACK, PUBREC,
    /// PUBREL or PUBCOMP packet. Note that this slice's length must be less than [`Client`]'s const
    /// generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    pub user_properties: &'a [MqttStringPair<'a>],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d> Options<'d> {
    /// Creates new acknowledgement options without properties.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            reason_string: None,
            user_properties: &[],
        }
    }

    /// Sets the reason string property.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub const fn reason_string(mut self, reason_string: MqttString<'d>) -> Self {
        self.reason_string = Some(reason_string);
        self
    }
    /// Sets the user properties. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    #[must_use]
    pub const fn user_properties(mut self, user_properties: &'d [MqttStringPair<'d>]) -> Self {
        self.user_properties = user_properties;
        self
    }
}

/// The mode with which acknowledgements of a publication flow for a given packet
/// identifier and an incoming/outgoing direction are sent. For more information,
/// check out the documentation of [`Client`].
///
/// [`Client`]: crate::client::Client
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Mode {
    /// All acknowledgements associated with a packet identifier of this mode are sent
    /// automatically by the client with [`ReasonCode::Success`]. In case of a
    /// reconnection, PUBLISH packets have to be resent manually individually and all
    /// due PUBREL packets are resent collectively with [`Client::rerelease`].
    ///
    /// [`ReasonCode::Success`]: crate::types::ReasonCode::Success
    /// [`Client::rerelease`]: crate::client::Client::rerelease
    #[default]
    Automatic,

    /// Most acknowledgements but a few exceptions must be sent manually by the user.
    /// See the documentation of [`Client`] for a detailed description of these cases.
    ///
    /// [`Client`]: crate::client::Client
    Manual,
}

impl Mode {
    /// Returns `true` if the ack mode is [`Automatic`].
    ///
    /// [`Automatic`]: crate::client::options::AckMode::Automatic
    #[must_use]
    pub fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic)
    }

    /// Returns `true` if the ack mode is [`Manual`].
    ///
    /// [`Manual`]: crate::client::options::AckMode::Manual
    #[must_use]
    pub fn is_manual(&self) -> bool {
        matches!(self, Self::Manual)
    }
}
