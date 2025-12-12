use crate::config::SessionExpiryInterval;

/// Configuration of the client which must be upheld by the server.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The session expiry interval requested by the client.
    /// This field is used to determine whether a non-zero session expiry interval
    /// can be used when disconnecting.
    pub session_expiry_interval: SessionExpiryInterval,
    // receive_maximum
    // maximum_packet_size
    // topic_alias_maximum
    // request_response_information this requests a response_information property in the CONNACK
    // request_problem_information indicates to the server that it may include reason strings and user properties
}
