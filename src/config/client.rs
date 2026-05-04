use crate::{config::SessionExpiryInterval, types::VarByteInt};

/// Configuration of the client which must be upheld by the server.
/// These values are used by the client to enforce protocol correctness.
///
/// Other values which might be added in the future here are:
/// * Client's receive maximum: this is currently a const generic of the client type
///   and is therefore not required as an in-memory value.
/// * Client's topic alias maximum: incoming topic aliases are currently not supported,
///   this is configured into the protocol via a topic alias maximum value of 0.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Config {
    /// The session expiry interval requested by the client. Note that this is NOT
    /// necessarily the actual session expiry interval value in force which is used by
    /// the server, as the server can overwrite this value in its CONNACK packet. The
    /// negotiated and applicable value is stored in [`SharedConfig`].
    ///
    /// This field is still required however to determine whether a non-zero session
    /// expiry interval can be used when disconnecting.
    ///
    /// This value is configured when calling [`Client::connect`] from the value in
    /// [`ConnectOptions`].
    ///
    /// [`SharedConfig`]: crate::config::SharedConfig
    /// [`Client::connect`]: crate::client::Client::connect
    /// [`ConnectOptions`]: crate::client::options::ConnectOptions
    pub session_expiry_interval: SessionExpiryInterval,

    /// The maximum packet size the client is willing to accept transformed into the
    /// maximum value of the remaining length that is therefore accepted.
    ///
    /// This value is configured when calling [`Client::connect`] from the value in
    /// [`ConnectOptions`].
    ///
    /// [`Client::connect`]: crate::client::Client::connect
    /// [`ConnectOptions`]: crate::client::options::ConnectOptions
    pub maximum_accepted_remaining_length: u32,

    /// Whether reason strings and user properties are sent (by the server) in case
    /// of failures. In reality, this depends a lot on the specific server implementation
    /// and only follows this rule:
    ///
    /// Whether the server is allowed to send reason strings or user properties on
    /// packets other than PUBLISH, CONNACK or DISCONNECT (PUBACK, PUBREC, PUBREL,
    /// PUBCOMP, SUBACK, UNSUBACK, AUTH).
    ///
    /// [MQTTv5.0 3.1.2.11.7](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901053)
    ///
    /// This value is configured when calling [`Client::connect`] from the value in
    /// [`ConnectOptions`].
    ///
    /// [`Client::connect`]: crate::client::Client::connect
    /// [`ConnectOptions`]: crate::client::options::ConnectOptions
    pub request_problem_information: bool,
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
