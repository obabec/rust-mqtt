use const_fn::const_fn;

use crate::{
    config::SessionExpiryInterval,
    types::{MqttString, MqttStringPair, ReasonCode},
};

/// Options for a disconnection to the server with a DISCONNECT packet.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'d> {
    /// The [`ReasonCode`] of the DISCONNECT packet.
    ///
    /// Must be one of [`ReasonCode::Success`], [`ReasonCode::DisconnectWithWillMessage`],
    /// [`ReasonCode::UnspecifiedError`], [`ReasonCode::MalformedPacket`],
    /// [`ReasonCode::ProtocolError`], [`ReasonCode::ImplementationSpecificError`],
    /// [`ReasonCode::TopicNameInvalid`], [`ReasonCode::ReceiveMaximumExceeded`],
    /// [`ReasonCode::TopicAliasInvalid`], [`ReasonCode::PacketTooLarge`],
    /// [`ReasonCode::MessageRateTooHigh`], [`ReasonCode::QuotaExceeded`],
    /// [`ReasonCode::AdministrativeAction`] or [`ReasonCode::PayloadFormatInvalid`]
    /// (Compare [Disconnect Reason Code](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901208), \[MQTT-3.14.2-1\]).
    pub reason_code: ReasonCode,

    /// The session expiry interval property. This value overrides the session expiry interval
    /// negotiated in the handshake.
    ///
    /// Must not to a non-zero value (`Some(`[`SessionExpiryInterval::EndOnDisconnect`]`)`) if the
    /// session expiry interval property in the CONNECT packet has been zero (can be checked via
    /// [`Client::client_config`]). The client will not disconnect if a violation occurs but prevent
    /// the protocol error and return an error.
    ///
    /// [`Client::client_config`]: crate::client::Client::client_config
    pub session_expiry_interval: Option<SessionExpiryInterval>,

    /// The reason string property of the DISCONNECT packet.
    pub reason_string: Option<MqttString<'d>>,

    /// Arbitrary key-value pairs of strings sent as the user property entries of the DISCONNECT
    /// packet. Note that this slice's length must be less than [`Client`]'s const generic parameter
    /// `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    pub user_properties: &'d [MqttStringPair<'d>],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d> Options<'d> {
    /// Creates new disconnect options with will publication disabled
    /// ([`ReasonCode::Success`] aka Normal Disconnection) and no session
    /// expiry interval.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            reason_code: ReasonCode::Success,
            session_expiry_interval: None,
            reason_string: None,
            user_properties: &[],
        }
    }

    /// Sets the reason code.
    ///
    /// Must be one of [`ReasonCode::Success`], [`ReasonCode::DisconnectWithWillMessage`],
    /// [`ReasonCode::UnspecifiedError`], [`ReasonCode::MalformedPacket`],
    /// [`ReasonCode::ProtocolError`], [`ReasonCode::ImplementationSpecificError`],
    /// [`ReasonCode::TopicNameInvalid`], [`ReasonCode::ReceiveMaximumExceeded`],
    /// [`ReasonCode::TopicAliasInvalid`], [`ReasonCode::PacketTooLarge`],
    /// [`ReasonCode::MessageRateTooHigh`], [`ReasonCode::QuotaExceeded`],
    /// [`ReasonCode::AdministrativeAction`] or [`ReasonCode::PayloadFormatInvalid`]
    /// (Compare [Disconnect Reason Code](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901208), \[MQTT-3.14.2-1\]).
    #[must_use]
    pub const fn reason_code(mut self, reason_code: ReasonCode) -> Self {
        self.reason_code = reason_code;
        self
    }
    /// Sets the reason code to [`ReasonCode::DisconnectWithWillMessage`].
    #[must_use]
    pub const fn publish_will(self) -> Self {
        self.reason_code(ReasonCode::DisconnectWithWillMessage)
    }
    /// Sets the session expiry interval property.
    #[must_use]
    pub const fn session_expiry_interval(mut self, interval: SessionExpiryInterval) -> Self {
        self.session_expiry_interval = Some(interval);
        self
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
