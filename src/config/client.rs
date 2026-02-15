use crate::{config::SessionExpiryInterval, types::VarByteInt};

/// Configuration of the client which must be upheld by the server.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The session expiry interval requested by the client.
    /// This field is used to determine whether a non-zero session expiry interval
    /// can be used when disconnecting.
    /// This value is configured when calling [`crate::client::Client::connect`]
    /// from the value in [`crate::client::options::ConnectOptions`]
    pub session_expiry_interval: SessionExpiryInterval,

    /// The maximum packet size the client is willing to accept transformed into the
    /// maximum value of the remaining length that is therefore accepted.
    /// This value is configured when calling [`crate::client::Client::connect`]
    /// from the value in [`crate::client::options::ConnectOptions`]
    pub maximum_accepted_remaining_length: u32,
    // receive_maximum
    // topic_alias_maximum
    // request_response_information this requests a response_information property in the CONNACK
    // request_problem_information indicates to the server that it may include reason strings and user properties
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session_expiry_interval: Default::default(),
            maximum_accepted_remaining_length: VarByteInt::MAX_ENCODABLE,
        }
    }
}
