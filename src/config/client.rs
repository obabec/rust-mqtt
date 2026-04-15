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

    /// Whether reason string and user properties are sent (by the server) in case
    /// of failures. In reality, this depends a lot on the specific server implementation
    /// and only follows this rule:
    ///
    /// Whether the server is allowed to send reason strings or user properties on
    /// packets other than PUBLISH, CONNACK or DISCONNECT (PUBACK, PUBREC, PUBREL,
    /// PUBCOMP, SUBACK, UNSUBACK, AUTH).
    ///
    /// [MQTTv5.0 3.1.2.11.7](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901053)
    ///
    /// This value is configured when calling [`crate::client::Client::connect`]
    /// from the value in [`crate::client::options::ConnectOptions`]
    pub request_problem_information: bool,
    // receive_maximum
    // topic_alias_maximum
    // request_response_information this requests a response_information property in the CONNACK
}

impl Default for Config {
    fn default() -> Self {
        Self {
            session_expiry_interval: SessionExpiryInterval::default(),
            maximum_accepted_remaining_length: VarByteInt::MAX_ENCODABLE,
            request_problem_information: false,
        }
    }
}
