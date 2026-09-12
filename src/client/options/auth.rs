use const_fn::const_fn;

use crate::types::{MqttBinary, MqttString, MqttStringPair};

/// Options for enhanced re-authentication for the AUTH packet.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'a> {
    /// The authentication data property of the AUTH packet.
    pub authentication_data: Option<MqttBinary<'a>>,

    /// The reason string property of the AUTH packet.
    pub reason_string: Option<MqttString<'a>>,

    /// Arbitrary key-value pairs of strings sent as the user property entries of the AUTH packet.
    /// Note that this slice's length must be less than [`Client`]'s const generic parameter
    /// `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    pub user_properties: &'a [MqttStringPair<'a>],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Options<'a> {
    /// Creates new authentication options without properties.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            authentication_data: None,
            reason_string: None,
            user_properties: &[],
        }
    }

    /// Sets the authentication data property.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub const fn authentication_data(mut self, authentication_data: MqttBinary<'a>) -> Self {
        self.authentication_data = Some(authentication_data);
        self
    }
    /// Sets the reason string property.
    #[const_fn(cfg(not(feature = "alloc")))]
    #[must_use]
    pub const fn reason_string(mut self, reason_string: MqttString<'a>) -> Self {
        self.reason_string = Some(reason_string);
        self
    }
    /// Sets the user properties. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    #[must_use]
    pub const fn user_properties(mut self, user_properties: &'a [MqttStringPair<'a>]) -> Self {
        self.user_properties = user_properties;
        self
    }
}
