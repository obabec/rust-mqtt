//! Contains the main `Event` and content types the client can emit.

use heapless::Vec;

use crate::{
    bytes::Bytes,
    client::AckMode,
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

    /// The server sent a PUBLISH packet. In the case of [`QoS::AtLeastOnce`], this can be
    /// a duplicate packet as indicated by the `DUP` flag, however, this flag being set
    /// does not rule out the possibility of this packet being the first one to deliver
    /// the application message. In the case of [`QoS::ExactlyOnce`], this packet and event
    /// is definitely the first one to deliver the application message despite the setting
    /// of the `DUP` flag, any other instances of the same application message will surface
    /// as [`Event::Duplicate`].
    ///
    /// The client has responded as follows:
    /// - [`QoS::AtMostOnce`]: No action
    /// - [`QoS::AtLeastOnce`] and [`Publish::ack_mode`] is [`AckMode::Automatic`]: A PUBACK packet has been sent to the server.
    /// - [`QoS::AtLeastOnce`] and [`Publish::ack_mode`] is [`AckMode::Manual`]: No action, the PUBACK must be sent manually by the user with [`Client::manual_acknowledge`].
    /// - [`QoS::ExactlyOnce`]: A PUBREC packet has been sent to the server.
    /// - [`QoS::ExactlyOnce`] and [`Publish::ack_mode`] is [`AckMode::Automatic`]: A PUBREC packet has been sent to the server.
    /// - [`QoS::ExactlyOnce`] and [`Publish::ack_mode`] is [`AckMode::Manual`]: No action, the PUBREC must be sent manually by the user with [`Client::manual_receive`].
    ///
    /// [`QoS::AtMostOnce`]: crate::types::QoS::AtMostOnce
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    /// [`Client::manual_acknowledge`]: crate::client::Client::manual_acknowledge
    /// [`Client::manual_receive`]: crate::client::Client::manual_receive
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

    /// The server sent a PUBACK, PUBREC or PUBCOMP with an erroneous [`ReasonCode`],
    /// therefore rejecting the publication. The publication process is aborted, the client
    /// has removed this publication's flight state from its session and has not responded
    /// with another packet. The publication can be retried with [`Client::publish`], note
    /// however that this retry is a new application message.
    ///
    /// The included [`ReasonCode`] is always erroneous.
    ///
    /// In the case of an erroneous PUBCOMP, the spec states:
    /// This is not an error during recovery, but at other times indicates a mismatch
    /// between the session state on the client and server.
    ///
    /// [`Client::publish`]: crate::client::Client::publish
    PublishRejected(Pubrej<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBREL with an erroneous [`ReasonCode`], therefore aborting its own
    /// publication. The reason code can only be [`ReasonCode::PacketIdentifierNotFound`].
    /// Note that the client has already delivered the associated publication previously with
    /// an [`Event::Publish`]. The client has automatically responded with a PUBCOMP packet as
    /// dictated by the specification.
    ///
    /// The spec states:
    /// This is not an error during recovery, but at other times indicates a mismatch
    /// between the session state on the client and server.
    ///
    /// [`QoS`]: crate::types::QoS
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    PublishAborted(Pubrej<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBACK packet matching a [`QoS::AtLeastOnce`] PUBLISH packet
    /// confirming that the PUBLISH has been received. The [`QoS::AtLeastOnce`]
    /// publication process is complete, the PUBLISH packet won't have to be resent.
    ///
    /// The included [`AckMode`] has no significance because for outgoing publications
    /// at [`QoS::AtLeastOnce`], there are no acknowledgements to be sent by the client.
    /// The included [`ReasonCode`] is always successful.
    ///
    /// [`QoS::AtLeastOnce`]: crate::types::QoS::AtLeastOnce
    PublishAcknowledged(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBREC packet matching a [`QoS::ExactlyOnce`] PUBLISH packet
    /// confirming that the PUBLISH has been received. The first handshake of the
    /// [`QoS::ExactlyOnce`] publication process is complete, the PUBLISH packet won't
    /// have to be resent. In case of [`AckMode::Automatic`], the client has responded
    /// with a PUBREL packet, for [`AckMode::Manual`], this PUBREL has to be sent
    /// manually via [`Client::manual_release`].
    ///
    /// The included [`ReasonCode`] is always successful.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    /// [`Client::manual_release`]: crate::client::Client::manual_release
    PublishReceived(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBREL packet matching a [`QoS::ExactlyOnce`] PUBREC packet
    /// confirming that the PUBREC has been received. The [`QoS::ExactlyOnce`]
    /// publication process is complete, the PUBREC packet won't have to be resent.
    /// In case of [`AckMode::Automatic`], the client has responded with a PUBCOMP
    /// packet, for [`AckMode::Manual`], this PUBCOMP has to be sent manually via
    /// [`Client::manual_complete`].
    ///
    /// The included [`ReasonCode`] is always successful.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    /// [`Client::manual_complete`]: crate::client::Client::manual_complete
    PublishReleased(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a PUBCOMP packet matching a [`QoS::ExactlyOnce`] PUBREL packet
    /// confirming that the PUBREL has been received. The [`QoS::ExactlyOnce`]
    /// publication process is complete, the PUBREL packet won't have to be resent.
    ///
    /// The included [`AckMode`] has no significance because the PUBCOMP packet
    /// completes the handshake removing the packet identifier from the session state
    /// completely, there are no more acknowledgements to be sent by the client.
    /// The included [`ReasonCode`] is always successful.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    PublishComplete(Puback<'e, MAX_USER_PROPERTIES>),

    /// The server sent a SUBACK, UNSUBACK, PUBACK, PUBREC, PUBREL or PUBCOMP
    /// packet with a packet identifier that is not in flight (anymore) or the
    /// server sent a PUBREC, PUBREL or PUBCOMP packet that did not drive the
    /// the session entry of its packet identifier forward.
    ///
    /// The client has not responded to the server or has responded
    /// automatically according to MQTT's rules to prevent a potential protocol
    /// deadlock. In both cases, no manual acknowledgements are required or
    /// allowed.
    Ignored,

    /// The server sent a [`QoS::ExactlyOnce`] PUBLISH packet which would cause a duplicate.
    /// The [`AckMode`] of the original PUBLISH packet for this packet identifier is unchanged,
    /// which is also the value set for the included [`Publish::ack_mode`] instead of the value
    /// produced by the predicate optionally set with [`Client::ack_manually_when`]. The client
    /// response behaviour depends on the original [`AckMode`] value (the one in this
    /// [`Publish::ack_mode`]):
    /// - [`AckMode::Automatic`]: The client has responded automatically.
    /// - [`AckMode::Manual`]:
    ///   - The client has responded automatically in all cases where a PUBREC was previously
    ///     sent within the same network connection.
    ///   - If a reconnection occured and this duplicate is the first republish, the PUBREC
    ///     packet must still be sent manually.
    ///
    /// Because the message was deserialized already anyway, it is included here, however, it
    /// is **NOT** a valid application message and **MUST** be treated like it wasn't ever
    /// delivered by the client.
    ///
    /// [`QoS::ExactlyOnce`]: crate::types::QoS::ExactlyOnce
    /// [`Client::ack_manually_when`]: crate::client::Client::ack_manually_when
    Duplicate(Publish<'e, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>),
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

/// Content of [`Event::Publish`] or [`Event::Duplicate`]. In the latter case, it is **NOT** a valid
/// application message and **MUST** be treated like it wasn't ever delivered by the client.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Publish<'p, const MAX_SUBSCRIPTION_IDENTIFIERS: usize, const MAX_USER_PROPERTIES: usize>
{
    /// The acknowledgement mode the client has determined with its given function to use for this
    /// publication flow.
    pub ack_mode: AckMode,

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
    /// The acknowledgement mode that was configured at the time of
    /// publication (for outgoing publications) or the mode the client has
    /// determined with its given function (for incoming publications).
    /// This value is unchanged from the same field in the associated,
    /// previous [`Publish`] or [`Puback`] event.
    pub ack_mode: AckMode,
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

impl<'p, const MAX_USER_PROPERTIES: usize> Puback<'p, MAX_USER_PROPERTIES> {
    pub(crate) fn new<T>(
        packet: GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>,
        ack_mode: AckMode,
    ) -> Self {
        debug_assert!(packet.reason_code.is_success());

        Self {
            ack_mode,
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
