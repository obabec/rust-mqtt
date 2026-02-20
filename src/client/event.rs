//! Contains the main `Event` and content types the client can emit.

use heapless::Vec;

use crate::{
    bytes::Bytes,
    types::{
        IdentifiedQoS, MqttBinary, MqttString, PacketIdentifier, ReasonCode, TopicName, VarByteInt,
    },
};

/// Events emitted by the client when receiving an MQTT packet.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Event<'e, const MAX_SUBSCRIPTION_IDENTIFIERS: usize> {
    /// The server sent a PINGRESP packet.
    Pingresp,

    /// The server sent a PUBLISH packet.
    ///
    /// The client has acted as follows:
    /// - QoS 0: No action
    /// - QoS 1: A PUBACK packet has been sent to the server.
    /// - QoS 2: A PUBREC packet has been sent to the server and the packet identifier is tracked as in flight
    Publish(Publish<'e, MAX_SUBSCRIPTION_IDENTIFIERS>),

    /// The server sent a SUBACK packet matching a SUBSCRIBE packet.
    ///
    /// The subscription process is complete and was successful if the reason code indicates success.
    /// The SUBSCRIBE packet won't have to be resent.
    Suback(Suback),

    /// The server sent an UNSUBACK packet matching an UNSUBSCRIBE packet.
    ///
    /// The unsubscription process is complete and was successful if the reason code indicates success.
    /// The UNSUBSCRIBE packet won't have to be resent.
    Unsuback(Suback),

    /// The server sent a PUBACK or PUBREC with an erroneous reason code,
    /// therefore rejecting the publication.
    ///
    /// The included reason code is always erroneous.
    ///
    /// The publication process is aborted.
    PublishRejected(Pubrej),

    /// The server sent a PUBACK packet matching a QoS 1 PUBLISH packet
    /// confirming that the PUBLISH has been received.
    ///
    /// The included reason code is always successful.
    ///
    /// The QoS 1 publication process is complete,
    /// the PUBLISH packet won't have to be resent.
    PublishAcknowledged(Puback),

    /// The server sent a PUBREC packet matching a QoS 2 PUBLISH packet
    /// confirming that the PUBLISH has been received.
    ///
    /// The included reason code is always successful.
    ///
    /// The client has responded with a PUBREL packet.
    ///
    /// The first handshake of the QoS 2 publication process is complete,
    /// the PUBLISH packet won't have to be resent.
    PublishReceived(Puback),

    /// The server sent a PUBREL packet matching a QoS 2 PUBREC packet
    /// confirming that the PUBREC has been received.
    ///
    /// The included reason code is always successful.
    ///
    /// The client has responded with a PUBCOMP packet.
    ///
    /// The QoS 2 publication process is complete,
    /// the PUBREC packet won't have to be resent.
    PublishReleased(Puback),

    /// The server sent a PUBCOMP packet matching a QoS 2 PUBREL packet
    /// confirming that the PUBREL has been received.
    ///
    /// The included reason code is always successful.
    ///
    /// The QoS 2 publication process is complete,
    /// the PUBREL packet won't have to be resent.
    PublishComplete(Puback),

    /// The server sent a SUBACK, PUBACK, PUBREC, PUBREL or PUBCOMP
    /// packet with a packet identifier that is not in flight (anymore).
    ///
    /// The client has not responded to the server or has responded appropriately
    /// to prevent a potential protocol deadlock.
    Ignored,

    /// The server sent a QoS 2 PUBLISH packet which would cause a duplicate.
    ///
    /// The client has responded with a PUBREC packet.
    Duplicate,
}

/// Content of [`Event::Suback`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Suback {
    /// Packet identifier of the acknowledged SUBSCRIBE packet.
    pub packet_identifier: PacketIdentifier,
    /// Reason code returned for the subscription.
    pub reason_code: ReasonCode,
}

/// Content of [`Event::Publish`].
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Publish<'p, const MAX_SUBSCRIPTION_IDENTIFIERS: usize> {
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
pub struct Puback {
    /// Packet identifier of the acknowledged PUBLISH packet.
    pub packet_identifier: PacketIdentifier,
    /// Reason code of this state in the publication process
    pub reason_code: ReasonCode,
}

/// Content of [`Event::PublishRejected`].
///
/// The reason code is always erroneous.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Pubrej {
    /// Packet identifier of the rejected PUBLISH packet.
    pub packet_identifier: PacketIdentifier,
    /// Reason code of the rejection.
    pub reason_code: ReasonCode,
}
