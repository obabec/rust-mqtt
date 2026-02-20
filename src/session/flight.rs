use crate::types::PacketIdentifier;

/// An incomplete QoS 1 or 2 publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct InFlightPublish<S> {
    /// The packet identifier of the publication process.
    pub packet_identifier: PacketIdentifier,
    /// The state of the publication process.
    pub state: S,
}

/// The state of an incomplete QoS 1 or 2 publication by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CPublishFlightState {
    /// A QoS 1 PUBLISH packet has been sent.
    /// The next step in the handshake is the server sending a PUBACK packet.
    AwaitingPuback,
    /// A QoS 2 PUBLISH packet has been sent.
    /// The next step in the handshake is the server sending a PUBREC packet.
    AwaitingPubrec,
    /// A PUBREC packet has been received and responded to with a PUBREL packet.
    /// The last step in the handshake is the server sending a PUBCOMP packet.
    AwaitingPubcomp,
}

/// The state of an incomplete QoS 2 publication by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SPublishFlightState {
    /// A QoS 2 packet has been received and responded to with a PUBREC packet.
    /// The next step in the handshake is the server sending a PUBREL packet.
    AwaitingPubrel,
}
