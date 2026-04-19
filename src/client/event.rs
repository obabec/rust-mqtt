//! Contains the main `Event` and content types the client can emit.

use heapless::Vec;

use crate::{
    bytes::Bytes,
    types::{
        IdentifiedQoS, MqttBinary, MqttString, MqttStringPair, PacketIdentifier, ReasonCode,
        TopicName, VarByteInt,
    },
    v5::{packet::GenericPubackPacket, property::Property},
};

/// Contains information taken from a connection handshake which the client does not have to
/// store for correct operational behaviour.
///
/// Does not include the [`ReasonCode`] as it is always [`ReasonCode::Success`]
/// (0x00) if this event is returned.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Connected<'i, const MAX_USER_PROPERTIES: usize> {
    /// If set to true, a previous session has been continued by the server for this connection.
    pub session_present: bool,

    /// The server can assign a different client identifier than the one in the CONNECT packet
    /// or must assign a client identifier if none was included in the CONNECT packet. This is
    /// the final client identifier value used for this session and connection.
    pub client_identifier: MqttString<'i>,

    /// The user property entries in the CONNACK packet. If the vector is full, this list might
    /// not be exhaustive.
    pub user_properties: Vec<MqttStringPair<'i>, MAX_USER_PROPERTIES>,

    /// Response information used to create response topics.
    pub response_information: Option<MqttString<'i>>,

    /// Another server which can be used.
    pub server_reference: Option<MqttString<'i>>,
}

/// Events emitted by the client when receiving an MQTT packet.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Event<'e, const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize> {
    /// The server sent a PINGRESP packet.
    Pingresp,

    /// The server sent a PUBLISH packet.
    ///
    /// The client has acted as follows:
    /// - [`QoS::AtMostOnce`]: No action
    /// - [`QoS::AtLeastOnce`]: A PUBACK packet has been sent to the server.
    /// - [`QoS::ExactlyOnce`]: A PUBREC packet has been sent to the server and the packet
    ///   identifier is tracked as in flight
    ///
    /// [`QoS::AtMostOnce`]: crate::types::QoS::AtMostOnce
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    Publish(Publish<'e, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>),

    /// The server sent a SUBACK packet matching a SUBSCRIBE packet.
    ///
    /// The subscription process is complete and was successful if the [`ReasonCode`] indicates
    /// success. The SUBSCRIBE packet won't have to be resent.
    Suback(Suback<'e, MAX_USER_PROPERTIES>),

    /// The server sent an UNSUBACK packet matching an UNSUBSCRIBE packet.
    ///
    /// The unsubscription process is complete and was successful if the [`ReasonCode`]
    /// indicates success. The UNSUBSCRIBE packet won't have to be resent.
    Unsuback(Suback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBACK or PUBREC with an erroneous [`ReasonCode`],
    /// therefore rejecting the publication.
    ///
    /// The included [`ReasonCode`] is always erroneous.
    ///
    /// The publication process is aborted.
    PublishRejected(Pubrej<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBACK packet matching a [`QoS::AtLeastOnce`] PUBLISH packet
    /// confirming that the PUBLISH has been received.
    ///
    /// The included [`ReasonCode`] is always successful.
    ///
    /// The [`QoS::AtLeastOnce`] publication process is complete,
    /// the PUBLISH packet won't have to be resent.
    ///
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    PublishAcknowledged(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBREC packet matching a [`QoS::ExactlyOnce`] PUBLISH packet
    /// confirming that the PUBLISH has been received.
    ///
    /// The included [`ReasonCode`] is always successful.
    ///
    /// The client has responded with a PUBREL packet.
    ///
    /// The first handshake of the [`QoS::ExactlyOnce`] publication process is complete,
    /// the PUBLISH packet won't have to be resent.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    PublishReceived(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBREL packet matching a [`QoS::ExactlyOnce`] PUBREC packet
    /// confirming that the PUBREC has been received.
    ///
    /// The included [`ReasonCode`] is always successful.
    ///
    /// The client has responded with a PUBCOMP packet.
    ///
    /// The [`QoS::ExactlyOnce`] publication process is complete,
    /// the PUBREC packet won't have to be resent.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    PublishReleased(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBCOMP packet matching a [`QoS::ExactlyOnce`] PUBREL packet
    /// confirming that the PUBREL has been received.
    ///
    /// The included [`ReasonCode`] is always successful.
    ///
    /// The [`QoS::ExactlyOnce`] publication process is complete,
    /// the PUBREL packet won't have to be resent.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    PublishComplete(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a SUBACK, PUBACK, PUBREC, PUBREL or PUBCOMP
    /// packet with a packet identifier that is not in flight (anymore).
    ///
    /// The client has not responded to the server or has responded appropriately
    /// to prevent a potential protocol deadlock.
    Ignored,

    /// The server sent a [`QoS::ExactlyOnce`] PUBLISH packet which would cause a duplicate.
    ///
    /// The client has responded with a PUBREC packet.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    Duplicate,
}

/// Content of [`Event::Suback`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Suback<'s, const MAX_USER_PROPERTIES: usize> {
    /// Packet identifier of the acknowledged SUBSCRIBE packet.
    pub packet_identifier: PacketIdentifier,

    /// The reason string of the SUBACK/UNSUBACK packet.
    pub reason_string: Option<MqttString<'s>>,
    /// The user property entries in the SUBACK/UNSUBACK packet.
    /// If the vector is full, this list might not be exhaustive.
    pub user_properties: Vec<MqttStringPair<'s>, MAX_USER_PROPERTIES>,

    /// Reason code returned for the subscription.
    pub reason_code: ReasonCode,
}

/// Content of [`Event::Publish`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Publish<'p, const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize>
{
    /// The DUP flag in the PUBLISH packet. If set to false, it indicates that this is the first occasion
    /// the server has attempted to send this publication.
    pub dup: bool,

    /// The quality of service the server determined to use for this publication. It is the minimum of
    /// the matching subscription with the highest quality of service level and the quality of service of
    /// the publishing client's publication.
    ///
    /// If the quality of service is greater than 0, this includes the non-zero packet identifier of the
    /// PUBLISH packet.
    pub identified_qos: IdentifiedQoS,

    /// The retain flag in the PUBLISH packet. If set to true, it indicates that the publication is the
    /// result of a retained message. If set to false, this publication having been retained depends on
    /// the retain as published flag of the matching subscription.
    pub retain: bool,

    /// The exact topic of this publication.
    pub topic: TopicName<'p>,

    /// If present, indicates whether the payload is UTF-8. This value is set by the publisher and is
    /// NOT verified by the client library.
    /// This is equal to the payload format indicator property of the PUBLISH packet.
    pub payload_format_indicator: Option<bool>,

    /// The message expiry interval in seconds.
    /// This is calculated by subtracting the elapsed time since the publish from the message expiry
    /// interval in original publication.
    pub message_expiry_interval: Option<u32>,

    /// Identifies an incoming publication as a request and specifies the topic which the response should
    /// be published on.
    pub response_topic: Option<TopicName<'p>>,

    /// Present in incoming requests and responses. In either case this is arbitrary binary data used for
    /// associating either the following response with this specific request or in case of a response,
    /// link back to the original request.
    pub correlation_data: Option<MqttBinary<'p>>,

    /// The user property entries in the PUBLISH packet. If the vector is full, this list might not be
    /// exhaustive.
    pub user_properties: Vec<MqttStringPair<'p>, MAX_USER_PROPERTIES>,

    /// The subscription identifiers in the PUBLISH packet. If the vector is full, this list might not
    /// be exhaustive.
    pub subscription_identifiers: Vec<VarByteInt, MAX_SUBSCRIPTION_IDENTIFIERS>,

    /// The content type property of the PUBLISH packet
    pub content_type: Option<MqttString<'p>>,

    /// The application message of this publication.
    pub message: Bytes<'p>,
}

/// Content of [`Event::PublishAcknowledged`], [`Event::PublishReceived`],
/// [`Event::PublishReleased`], and [`Event::PublishComplete`].
///
/// The reason code is always successful.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Puback<'p, const MAX_USER_PROPERTIES: usize> {
    /// Packet identifier of the acknowledged PUBLISH packet.
    pub packet_identifier: PacketIdentifier,
    /// Reason code of this state in the publication process
    pub reason_code: ReasonCode,
    /// The reason string of the PUBACK/PUBREC/PUBREL/PUBCOMP packet.
    pub reason_string: Option<MqttString<'p>>,
    /// The user property entries in the PUBACK/PUBREC/PUBREL/PUBCOMP packet.
    /// If the vector is full, this list might not be exhaustive.
    pub user_properties: Vec<MqttStringPair<'p>, MAX_USER_PROPERTIES>,
}

impl<'p, T, const MAX_USER_PROPERTIES: usize> From<GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>>
    for Puback<'p, MAX_USER_PROPERTIES>
{
    fn from(packet: GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>) -> Self {
        debug_assert!(packet.reason_code.is_success());

        Self {
            packet_identifier: packet.packet_identifier,
            reason_code: packet.reason_code,
            reason_string: packet.reason_string.map(Property::into_inner),
            user_properties: packet
                .user_properties
                .into_iter()
                .map(Property::into_inner)
                .collect(),
        }
    }
}

/// Content of [`Event::PublishRejected`].
///
/// The reason code is always erroneous.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Pubrej<'p, const MAX_USER_PROPERTIES: usize> {
    /// Packet identifier of the rejected PUBLISH packet.
    pub packet_identifier: PacketIdentifier,
    /// Reason code of the rejection.
    pub reason_code: ReasonCode,
    /// The reason string of the PUBACK/PUBREC/PUBREL/PUBCOMP packet.
    pub reason_string: Option<MqttString<'p>>,
    /// The user property entries in the PUBACK/PUBREC/PUBREL/PUBCOMP packet.
    /// If the vector is full, this list might not be exhaustive.
    pub user_properties: Vec<MqttStringPair<'p>, MAX_USER_PROPERTIES>,
}

impl<'p, T, const MAX_USER_PROPERTIES: usize> From<GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>>
    for Pubrej<'p, MAX_USER_PROPERTIES>
{
    fn from(packet: GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>) -> Self {
        debug_assert!(packet.reason_code.is_erroneous());

        Self {
            packet_identifier: packet.packet_identifier,
            reason_code: packet.reason_code,
            reason_string: packet.reason_string.map(Property::into_inner),
            user_properties: packet
                .user_properties
                .into_iter()
                .map(Property::into_inner)
                .collect(),
        }
    }
}
