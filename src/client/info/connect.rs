use heapless::Vec;

use crate::types::{MqttString, MqttStringPair};

/// Information taken from a connection handshake the client does not have to store
/// for correct operational behaviour and does not store for optimization purposes.
///
/// Does not include the reason code as it is always [`crate::types::ReasonCode::Success`] (0x00)
/// if this is returned.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Info<'i, const MAX_USER_PROPERTIES: usize> {
    /// If set to true, a previous session is continued by the server for this connection.
    pub session_present: bool,

    /// The server can assign a different client identifier than the one in the CONNECT packet
    /// or must assign a client identifier if none was included in the CONNECT packet.
    /// This is the final client identifier value used for this session.
    pub client_identifier: MqttString<'i>,

    /// The user property entries in the CONNACK packet. If the vector is full, this list might
    /// not be exhaustive.
    pub user_properties: Vec<MqttStringPair<'i>, MAX_USER_PROPERTIES>,

    /// Response information used to create response topics.
    pub response_information: Option<MqttString<'i>>,

    /// Another server which can be used.
    pub server_reference: Option<MqttString<'i>>,
}
