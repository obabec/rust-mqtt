use crate::types::MqttStringPair;

/// Options for unsubscription included for every topic.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options<'s> {
    /// Arbitrary key-value pairs of strings sent as the user property entries of the
    /// UNSUBSCRIBE packet. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    pub user_properties: &'s [MqttStringPair<'s>],
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'s> Options<'s> {
    /// Creates options with values coherent to the [`Default`] implementations of the fields.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            user_properties: &[],
        }
    }

    /// Sets the user properties. Note that this slice's length must be less than [`Client`]'s
    /// const generic parameter `MAX_USER_PROPERTIES`.
    ///
    /// [`Client`]: crate::client::Client
    #[must_use]
    pub const fn user_properties(mut self, user_properties: &'s [MqttStringPair<'s>]) -> Self {
        self.user_properties = user_properties;
        self
    }
}
