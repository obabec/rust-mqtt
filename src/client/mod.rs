//! Implements full client functionality with session and configuration handling and Quality of Service flows.

use core::{matches, num::NonZero};

use crate::{
    buffer::BufferProvider,
    bytes::Bytes,
    client::{
        event::{Connected, Event, Puback, Publish, Pubrej, Suback},
        options::{
            AckMode, AckOptions, ConnectOptions, DisconnectOptions, PublicationOptions,
            SubscriptionOptions, TopicReference, UnsubscriptionOptions,
        },
        raw::Raw,
    },
    config::{ClientConfig, MaximumPacketSize, ServerConfig, SessionExpiryInterval, SharedConfig},
    fmt::{assert, const_assert, debug, error, info, panic, trace, unreachable, warn},
    header::{FixedHeader, PacketType},
    io::Transport,
    packet::{Packet, TxPacket},
    session::{Error as SmError, Event as SmEvent, LocalPublishState, Response, Session},
    types::{
        IdentifiedQoS, MqttBinary, MqttString, MqttStringPair, PacketIdentifier, QoS, ReasonCode,
        NoLocalSharedSubscription, SubscriptionFilter, TopicFilter, TopicName, VarByteInt,
    },
    v5::{
        packet::{
            ConnackPacket, ConnectPacket, DisconnectPacket, PingreqPacket, PingrespPacket,
            PubackPacket, PubcompPacket, PublishPacket, PubrecPacket, PubrelPacket, SubackPacket,
            SubscribePacket, UnsubackPacket, UnsubscribePacket,
        },
        property::Property,
    },
};

mod err;

pub mod event;
pub mod options;
pub mod raw;

pub use err::Error as MqttError;

/// An MQTT client.
///
/// Configuration via const parameters:
///
/// - `SUBSCRIBE_MAXIMUM`: The maximum amount of in-flight/unacknowledged packets allowed for SUBSCRIBE and UNSUBSCRIBE packets
///   each (one slot per call to [`Client::subscribe`] or [`Client::unsubscribe`]). Must not be greater than 65535.
/// - `RECEIVE_MAXIMUM`: MQTT's control flow mechanism. The maximum amount of incoming [`QoS::AtLeastOnce`] and
///   [`QoS::ExactlyOnce`] publications (accumulated). Must not be 0 and must not be greater than 65535.
/// - `SEND_MAXIMUM`: The maximum amount of outgoing [`QoS::AtLeastOnce`] and [`QoS::ExactlyOnce`] publications. The server
///   can further limit this with its receive maximum. The client will use the minimum of this value and [`Self::server_config`].
/// - `MAX_SUBSCRIPTION_IDENTIFIERS`: The maximum amount of subscription identifier properties the client can receive within a
///   single PUBLISH packet. If a packet with more subscription identifiers is received, the later identifers will be discarded.
/// - `MAX_USER_PROPERTIES`: The maximum amount of user properties that the client can send and receive in one packet.
///   - Must not be greater than 1021. This limitation currently exists to easily rule out any variable byte integer overflows.
///   - It is recommended (but not strictly required) to use a value >= 1, because if the value is 0, the client does not
///     guarantee to detect the protocol error and disconnect from the server when the request problem information property in
///     CONNECT is 0 and the server sends user properties in a packet other than CONNACK, DISCONNECT or PUBLISH.
///
/// The client has two modes of how the acknowledgements within handshakes of [`QoS::AtLeastOnce`] and [`QoS::ExactlyOnce`]
/// publications are handled. These modes are the default [`AckMode::Automatic`] and the proactively configurable
/// [`AckMode::Manual`]. The modes are applied separately and independently for each tuple of packet identifier and publication
/// direction (incoming or outgoing publication). For outgoing publications, the [`AckMode`] is selected once at the time of
/// sending the first PUBLISH packet with [`Client::publish`] and stays unchanged for this tuple (packet identifier/outgoing)
/// for the entire duration of the handshake until its completion and deletion from the session state after receiving a PUBCOMP
/// (Note that for outgoing publications, [`AckMode::Manual`] is only an option for [`QoS::ExactlyOnce`] and not for
/// [`QoS::AtLeastOnce`] publications because no acknowledgements are required by the client, and in case of a reconnection the
/// PUBLISH packet has to be retransmitted manually anyway).
/// For incoming publications, a predicate can be set which will be called for every applicable incoming PUBLISH packet and
/// determine the [`AckMode`] for this tuple (packet identifier/incoming). As with outgoing publications, this [`AckMode`] also
/// remains unchangd for the duration of the handshake until it's completed with an outgoing PUBACK, PUBCOMP or erroneous reason-
/// coded PUBREC packet or an incoming erroneous PUBREL packet.
/// Hard disconnections after which the session is not continued of course also clear any session entries and as such the
/// associations of packet identifiers and publication direction with an [`AckMode`] are also removed.
///
/// The state machine behind the publication mechanism is robust and prevents illegal acknowledgements or retransmissions and
/// therefore protocol errors triggered by incorrect calls to [`Client::republish`], [`Client::rerelease`],
/// [`Client::manual_acknowledge`], [`Client::manual_receive`], [`Client::manual_release`] and [`Client::manual_complete`].
/// By interpreting the specification liberally, the state machine accommodates unexpected server packets that a stricter
/// implementation might reject, raising errors only when there is no appropriate way to react.
///
/// What follows is an overview of over the automatic and manual acknowledgement modes, what work the client does automatically
/// vs what has to be done by the user and which acknowledgement packets are sent how.
///
/// - Outgoing [`QoS::AtMostOnce`] publication: No acknowledgement packets and retransmissions, only [`AckMode::Automatic`] is
///   available. After sending the fire-and-forget PUBLISH packet, no further action is required.
/// - Outgoing [`QoS::AtLeastOnce`] publication: No acknowledgement packets sent by the client, only [`AckMode::Automatic`] is
///   available.
///   - If a PUBACK packet has not been received yet when a reconnection happens, the PUBLISH packet must be resent with
///     [`Client::republish`].
/// - Outgoing [`QoS::ExactlyOnce`] publication:
///   - [`AckMode::Automatic`]:
///     - Upon reconnection, if a PUBREC packet has not been received yet, the PUBLISH packet must be resent with
///       [`Client::republish`].
///     - Upon reconnection, if a PUBREC packet has been received, the PUBREL packet must be retransmitted. This is done
///       with a single call to [`Client::rerelease`] which handles this case for all matching outgoing publications at once.
///     - All other acknowledgement packets not matching the criteria above are sent automatically.
///   - [`AckMode::Manual`]:
///     - When the first PUBREC packet is received (emitted as [`Event::PublishReceived`]), the user must manually send the
///       subsequent PUBREL with [`Client::manual_release`].
///     - Other received PUBREC packets (emitted as [`Event::Ignored`]) are acknowledged automatically.
///     - Upon reconnection, if a PUBREC packet has not been received yet, the PUBLISH packet must be resent with
///       [`Client::republish`].
///     - Upon reconnection, if a PUBREC packet has been received, the PUBREL packet must be (re-)transmitted with
///       [`Client::manual_release`]. Note that [`Client::rerelease`] will **NOT** match this criterion and won't send a PUBREL
///       for this packet identifier.
///
/// - Incoming [`QoS::AtMostOnce`] publication: No acknowledgement or retransmission, the [`AckMode`] deduced from the
///   predicate, if called, has no effect. After receiving the PUBLISH packet, no further action is required.
/// - Incoming [`QoS::AtLeastOnce`] publication:
///   - [`AckMode::Automatic`]: The client responds automatically with a PUBACK packet and removes the session state. In case of
///     a network failure during the transmission of the PUBACK, this behaviour is unchanged as the server must retransmit the
///     PUBLISH. No further action is required.
///   - [`AckMode::Manual`]:
///     - When a PUBLISH packet is received for a packet identifier for the first time in a network connection, the PUBACK packet
///       must be sent manually with [`Client::manual_acknowledge`]. This completes the handshake, subsequent PUBLISH packets are
///       treated as new application messages.
///     - When a PUBLISH packet has not been responded to with a PUBACK yet and a PUBLISH packet with the same packet identifier
///       is received (on the same or a different network connection, emitted as [`Event::Publish`]), a single call to
///       [`Client::manual_acknowledge`] completes the handshake.
/// - Incoming [`QoS::ExactlyOnce`] publication:
///   - [`AckMode::Automatic`]:
///     - When a PUBLISH packet is received, it is automatically acknowledged with a PUBREC packet.
///     - When a PUBREL packet is received, it is automatically acknowleged with a PUBCOMP packet.
///   - [`AckMode::Manual`]:
///     - When a PUBLISH packet is received for a packet identifier for the first time in a network connection, the PUBREC packet
///       must be sent manually with [`Client::manual_receive`].
///     - If a duplicate PUBLISH packet is received ([`Event::Duplicate`]) on the same network connection as the previous PUBLISH
///       and no PUBREC has been sent yet, the required call to [`Client::manual_receive`] acknowledges all PUBLISH packets
///       received up to this point.
///     - If a duplicate PUBLISH packet is received ([`Event::Duplicate`]) on the same network connection as the previous PUBLISH
///       packet and this previous PUBLISH packet has already been acknowledged with a PUBREC, the client acknowledges this
///       duplicate PUBLISH automatically.
///     - Upon reconnection, if a duplicate PUBLISH packet is received ([`Event::Duplicate`]), but a PUBREL has also been received
///       before, a protocol error is caused by the server. No manual acknowledgement is necessary immediately. After a
///       reconnection, the next steps are treated exactly as without a protocol error (but with a reconnection in mind).
///     - Upon reconnection, if a duplicate PUBLISH packet is received ([`Event::Duplicate`]) and no PUBREL has been received
///       before, a manual acknowledgement via [`Client::manual_receive`] is required independent of a PUBREC having been sent
///       before or not.
///     - When a PUBREL packet is received and it matches an entry, the PUBCOMP packet must be sent manually with
///       [`Client::manual_complete`]. This completes the handshake
///
/// - Only PUBLISH packets can instantiate a session entry with an associated [`AckMode`]. For this reason, when acknowledgement
///   packets with unused packet identifiers that require a responding packet are received (PUBREC and PUBREL), no session entry
///   is created and the responding packet (PUBREL and PUBCOMP) is sent automatically by the client.
pub struct Client<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
> {
    client_config: ClientConfig,
    shared_config: SharedConfig,
    server_config: ServerConfig,
    session: Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,

    raw: Raw<'c, N, B>,

    manual_ack_when:
        &'c dyn Fn(&Publish<'_, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>) -> bool,
}

impl<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
> core::fmt::Debug
    for Client<
        'c,
        N,
        B,
        SUBSCRIBE_MAXIMUM,
        RECEIVE_MAXIMUM,
        SEND_MAXIMUM,
        MAX_SUBSCRIPTION_IDENTIFIERS,
        MAX_USER_PROPERTIES,
    >
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Client")
            .field("client_config", &self.client_config)
            .field("shared_config", &self.shared_config)
            .field("server_config", &self.server_config)
            .field("session", &self.session)
            .field("raw", &self.raw)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "defmt")]
impl<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
> defmt::Format
    for Client<
        'c,
        N,
        B,
        SUBSCRIBE_MAXIMUM,
        RECEIVE_MAXIMUM,
        SEND_MAXIMUM,
        MAX_SUBSCRIPTION_IDENTIFIERS,
        MAX_USER_PROPERTIES,
    >
{
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "Client {{ client_config: {:?}, shared_config: {:?}, server_config: {:?}, session: {:?}, raw: {:?}, .. }}",
            self.client_config,
            self.shared_config,
            self.server_config,
            self.session,
            self.raw,
        );
    }
}

impl<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
>
    Client<
        'c,
        N,
        B,
        SUBSCRIBE_MAXIMUM,
        RECEIVE_MAXIMUM,
        SEND_MAXIMUM,
        MAX_SUBSCRIPTION_IDENTIFIERS,
        MAX_USER_PROPERTIES,
    >
{
    /// Creates a new, disconnected MQTT client using a buffer provider to store
    /// dynamically sized fields of received packets.
    /// The session state is initialised as a new session. If you want to start the
    /// client with an existing session, use [`Self::with_session`].
    /// All publications and acknowledgements will be acknowledged automatically.
    pub fn new(buffer: &'c mut B) -> Self {
        const {
            const_assert!(
                SUBSCRIBE_MAXIMUM <= 65535,
                "SUBSCRIBE_MAXIMUM must be less than or equal to 65535"
            );
            const_assert!(
                RECEIVE_MAXIMUM <= 65535,
                "RECEIVE_MAXIMUM must be less than or equal to 65535"
            );
            const_assert!(
                RECEIVE_MAXIMUM > 0,
                "RECEIVE_MAXIMUM must be greater than 0"
            );
            const_assert!(
                MAX_USER_PROPERTIES <= 1021,
                "MAX_USER_PROPERTIES must be less than or equal to 1021"
            );
        }

        Self {
            client_config: ClientConfig::default(),
            shared_config: SharedConfig::default(),
            server_config: ServerConfig::default(),
            session: Session::default(),
            raw: Raw::new_disconnected(buffer),

            manual_ack_when: &|_| false,
        }
    }

    /// Creates a new, disconnected MQTT client using a buffer provider to store
    /// dynamically sized fields of received packets.
    pub fn with_session(
        session: Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,
        buffer: &'c mut B,
    ) -> Self {
        let mut s = Self::new(buffer);
        s.session = session;
        s
    }

    /// Sets the predicate which selects whether the quality of service handshakes of an incoming
    /// publication are executed automatically by the client or manually by the user. If the
    /// predicate returns [`false`] for an incoming [`QoS::AtLeastOnce`] or [`QoS::ExactlyOnce`]
    /// PUBLISH packet, its handshake flow will be acknowledged automatically. If the predicate
    /// returns [`true`], the acknowledgement packets of its handshake must be sent manually mostly.
    /// Refer to the documentation of [`Client`] for a detailed description of these cases.
    pub fn ack_manually_when(
        &mut self,
        predicate: &'c dyn Fn(
            &Publish<'_, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>,
        ) -> bool,
    ) {
        self.manual_ack_when = predicate;
    }

    /// Returns the amount of publications the client is allowed to make according to the server's
    /// receive maximum. Does not account local space for storing publication state.
    fn remaining_send_quota(&self) -> u16 {
        self.server_config.receive_maximum.get() - self.session.active_outbound_publishes()
    }

    /// Returns configuration for this client.
    #[inline]
    pub fn client_config(&self) -> &ClientConfig {
        &self.client_config
    }

    /// Returns the configuration of the currently or last connected server if there is one.
    #[inline]
    pub fn server_config(&self) -> &ServerConfig {
        &self.server_config
    }

    /// Returns the configuration negotiated between the client and server.
    #[inline]
    pub fn shared_config(&self) -> &SharedConfig {
        &self.shared_config
    }

    /// Returns session related configuration and tracking information.
    #[inline]
    pub fn session(&self) -> &Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM> {
        &self.session
    }

    /// Returns an immutable reference to the supplied [`BufferProvider`] implementation.
    #[inline]
    pub fn buffer(&self) -> &B {
        self.raw.buffer()
    }

    /// Returns a mutable reference to the supplied [`BufferProvider`] implementation.
    ///
    /// This can for example be used to reset the underlying buffer if using `BumpBuffer`.
    #[inline]
    pub fn buffer_mut(&mut self) -> &mut B {
        self.raw.buffer_mut()
    }

    /// Connect the client to an MQTT server on the other end of the `net` argument.
    /// Sends a CONNECT message and awaits the CONNACK response by the server.
    ///
    /// Only call this when
    /// - the client is newly constructed.
    /// - a non-recoverable error has occured and [`Self::abort`] has been called.
    /// - [`Self::disconnect`] has been called.
    ///
    /// The session expiry interval in [`ConnectOptions`] overrides the one in the session of the client.
    ///
    /// Configuration that was negotiated with the server is stored in the `client_config`,
    /// `server_config`, `shared_config`, and `session` fields, which have getters
    /// ([`Self::client_config`], [`Self::server_config`], [`Self::shared_config`],
    /// [`Self::session`]).
    ///
    /// If the server does not have a session present, the client's session is cleared. In case you would want
    /// to keep the session state, you can call [`Self::session`] and clone the session before.
    ///
    /// # Returns:
    /// Information about the session/connection that the client does currently not use and therefore  not store
    /// in its configuration fields.
    ///
    /// # Errors
    ///
    /// * [`MqttError::Server`] if:
    ///   * the server sends a malformed packet
    ///   * the first received packet is something other than a CONNACK packet
    ///   * `client_identifier` is [`None`] and the server did not assign a client identifier
    ///   * the server causes a protocol error
    ///   * the server sends Response Information despite `request_response_information` in [`ConnectOptions`]
    ///     being 0
    /// * [`MqttError::Disconnect`] if the CONNACK packet's reason code is not successful (>= 0x80)
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::Alloc`] if the underlying [`BufferProvider`] returned an error
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`ConnectOptions`]
    /// or the length of the `user_properties` slice in the will in [`ConnectOptions`] is greater
    /// than `MAX_USER_PROPERTIES`.
    pub async fn connect<'d>(
        &mut self,
        net: N,
        options: &ConnectOptions<'_>,
        client_identifier: Option<MqttString<'d>>,
    ) -> Result<Connected<'d, MAX_USER_PROPERTIES>, MqttError<'c, MAX_USER_PROPERTIES>>
    where
        'c: 'd,
    {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send CONNECT with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );
        if let Some(ref will) = options.will {
            assert!(
                will.user_properties.len() <= MAX_USER_PROPERTIES,
                "attempted to send Will with {} > {} (MAX_USER_PROPERTIES) properties",
                will.user_properties.len(),
                MAX_USER_PROPERTIES
            );
        }

        self.raw.set_net(net);

        // Set client session expiry interval because it is relevant to determine
        // which session expiry interval can be sent in DISCONNECT packet.
        self.client_config.session_expiry_interval = options.session_expiry_interval;

        // Set request problem information because it is required to detect protocol
        // errors when server sends a reason string or user properties in any packet
        // other than PUBLISH, CONNACK, or DISCONNECT
        self.client_config.request_problem_information = options.request_problem_information;

        // Empirical maximum packet size mapping
        // -------------------------------------------------------------------------------------------------------
        //         remaining length              | fixed header length |              max packet size
        //                               0..=127 |                   2 |                                   2..=129
        //                          128..=16_383 |                   3 |                              131..=16_386
        //                    16_384..=2_097_151 |                   4 |                        16_388..=2_097_155
        // 2_097_152..=VarByteInt::MAX_ENCODABLE |                   5 | 2_097_157..=(VarByteInt::MAX_ENCODABLE+5)

        const MAX_POSSIBLE_PACKET_SIZE: u32 = VarByteInt::MAX_ENCODABLE + 5;

        self.client_config.maximum_accepted_remaining_length = match options.maximum_packet_size {
            MaximumPacketSize::Unlimited => u32::MAX,
            MaximumPacketSize::Limit(l) => match l.get() {
                0 => unreachable!("NonZero invariant"),
                1 => panic!(
                    "every MQTT packet is at least 2 bytes long, a smaller maximum packet size makes no sense"
                ),
                2..=129 => l.get() - 2,
                130..=16_386 => l.get() - 3,
                16_387..=2_097_155 => l.get() - 4,
                2_097_156..MAX_POSSIBLE_PACKET_SIZE => l.get() - 5,
                MAX_POSSIBLE_PACKET_SIZE.. => VarByteInt::MAX_ENCODABLE,
            },
        };

        trace!(
            "maximum accepted remaining length set to {:?}",
            self.client_config.maximum_accepted_remaining_length
        );

        {
            let packet_client_identifier = client_identifier
                .as_ref()
                .map(MqttString::as_borrowed)
                .unwrap_or_default();

            let mut packet = ConnectPacket::<MAX_USER_PROPERTIES>::new(
                packet_client_identifier,
                options.clean_start,
                options.keep_alive,
                options.maximum_packet_size,
                options.session_expiry_interval,
                // Safety: `Self::new` panics if `RECEIVE_MAXIMUM` is 0. Thus, this
                // code is only reached when `RECEIVE_MAXIMUM` is greater than 0.
                unsafe { NonZero::new_unchecked(RECEIVE_MAXIMUM as u16) },
                options.request_response_information,
                options.request_problem_information,
                options
                    .user_properties
                    .iter()
                    .map(MqttStringPair::as_borrowed)
                    .map(Into::into)
                    .collect(),
            );

            if let Some(ref user_name) = options.user_name {
                packet.add_user_name(user_name.as_borrowed());
            }
            if let Some(ref password) = options.password {
                packet.add_password(password.as_borrowed());
            }

            if let Some(ref will) = options.will {
                let will_qos = will.will_qos;
                let will_retain = will.will_retain;

                packet.add_will(will.as_borrowed_will(), will_qos, will_retain);
            }

            debug!("sending CONNECT packet");
            self.raw.send(&packet).await?;
            self.raw.flush().await?;
        }

        let header = self.raw.recv_header().await?;

        match header.packet_type() {
            Ok(ConnackPacket::<MAX_USER_PROPERTIES>::PACKET_TYPE) => debug!(
                "received CONNACK packet header (remaining length: {})",
                header.remaining_len.value()
            ),
            Ok(t) => {
                error!("received unexpected {:?} packet header", t);

                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::Server);
            }
            Err(_) => {
                error!("received invalid header {:?}", header);
                self.raw.close_with(Some(ReasonCode::MalformedPacket));
                return Err(MqttError::Server);
            }
        }

        let ConnackPacket::<MAX_USER_PROPERTIES> {
            session_present,
            reason_code,
            session_expiry_interval,
            receive_maximum,
            maximum_qos,
            retain_available,
            maximum_packet_size,
            assigned_client_identifier,
            topic_alias_maximum,
            reason_string,
            user_properties,
            wildcard_subscription_available,
            subscription_identifier_available,
            shared_subscription_available,
            server_keep_alive,
            response_information,
            server_reference,
        } = self.raw.recv_body(&header).await?;

        if !options.request_response_information && response_information.is_some() {
            error!("server sent response information when request response information was false");
            self.raw.close_with(Some(ReasonCode::ProtocolError));
            return Err(MqttError::Server);
        }

        if reason_code.is_success() {
            debug!("CONNACK packet indicates success");

            let client_identifier = assigned_client_identifier
                .map(Property::into_inner)
                .or(client_identifier)
                .ok_or_else(|| {
                    error!("server did not assign a client identifier when it was required.");
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    MqttError::Server
                })?;

            if session_present {
                if options.clean_start {
                    error!("server set the session present flag when clean start was set");
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                } else {
                    info!("connected to server and reconnected to session");
                    self.session.reconnect();
                }
            } else {
                #[allow(clippy::if_same_then_else)]
                if options.clean_start {
                    info!("connected to server");
                } else {
                    info!(
                        "connected to server but server does not have the requested session present"
                    );
                }
                self.session.clear();
            }

            self.shared_config.session_expiry_interval =
                session_expiry_interval.unwrap_or(options.session_expiry_interval);
            self.shared_config.keep_alive =
                server_keep_alive.map_or(options.keep_alive, Property::into_inner);

            if let Some(r) = receive_maximum {
                self.server_config.receive_maximum = r.into_inner();
            }
            if let Some(m) = maximum_qos {
                self.server_config.maximum_qos = m.into_inner();
            }
            if let Some(r) = retain_available {
                self.server_config.retain_supported = r.into_inner();
            }
            if let Some(m) = maximum_packet_size {
                self.server_config.maximum_packet_size = m;
            }
            if let Some(t) = topic_alias_maximum {
                self.server_config.topic_alias_maximum = t.into_inner();
            }
            if let Some(w) = wildcard_subscription_available {
                self.server_config.wildcard_subscription_supported = w.into_inner();
            }
            if let Some(s) = subscription_identifier_available {
                self.server_config.subscription_identifiers_supported = s.into_inner();
            }
            if let Some(s) = shared_subscription_available {
                self.server_config.shared_subscription_supported = s.into_inner();
            }

            Ok(Connected {
                session_present,
                client_identifier,
                user_properties: user_properties
                    .into_iter()
                    .map(Property::into_inner)
                    .collect(),
                response_information: response_information.map(Property::into_inner),
                server_reference: server_reference.map(Property::into_inner),
            })
        } else {
            debug!("CONNACK packet indicates rejection");
            info!("connection rejected by server (reason: {:?})", reason_code);

            self.raw.close_with(None);

            info!("disconnected from server");

            Err(MqttError::Disconnect {
                reason: reason_code,
                reason_string: reason_string.map(Property::into_inner),
                user_properties: user_properties
                    .into_iter()
                    .map(Property::into_inner)
                    .collect(),
                server_reference: server_reference.map(Property::into_inner),
            })
        }
    }

    /// Start a ping handshake by sending a PINGRESP packet.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    pub async fn ping(&mut self) -> Result<(), MqttError<'c, 0>> {
        debug!("sending PINGREQ packet");

        // PINGREQ has length 2 which really shouldn't exceed server's max packet size.
        // If it does the server should reconsider its incarnation as an MQTT server.
        self.raw.send(&PingreqPacket::new()).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Subscribes to a single topic with the given options.
    ///
    /// The client keeps track of the packet identifier sent in the SUBSCRIBE packet.
    /// If no [`Event::Suback`] is received within a custom time,
    /// this method can be used to send the SUBSCRIBE packet again.
    ///
    /// Note:
    /// * A topic filter with one or more wildcards should only be used if the server
    ///   supports wildcard subscriptions.
    /// * A subscription identifier should only be set if the server supports
    ///   subscription identifiers.
    /// * A topic filter of a shared subscriptions should only be used if the server
    ///   supports shared subscriptions.
    ///
    /// The server support of these requirements can be checked via [`Client::server_config`].
    /// If a violation occurs, the client will not subscribe but prevent the protocol error
    /// and return an error.
    ///
    /// # Returns:
    /// The packet identifier of the sent SUBSCRIBE packet.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::SessionBuffer`] if the buffer for outgoing SUBSCRIBE packet identifiers is full
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this SUBSCRIBE packet
    /// * [`MqttError::UnsupportedByServer`]
    ///   * if the server specified in its CONNACK that wildcard subscriptions are not available and
    ///     the topic filter is the topic filter of a shared subscription
    ///   * if the server specified in its CONNACK that subscription identifiers are not available and
    ///     the [`SubscriptionOptions`] include a subscription identifier
    ///   * if the server specified in its CONNACK that shared subscriptions are not available and the
    ///     topic filter is the topic filter of a shared subscription
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`SubscriptionOptions`]
    /// is greater than `MAX_USER_PROPERTIES`.
    pub async fn subscribe(
        &mut self,
        topic_filter: TopicFilter<'_>,
        options: &SubscriptionOptions<'_>,
    ) -> Result<PacketIdentifier, MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send SUBSCRIBE with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        if !self.server_config.wildcard_subscription_supported && topic_filter.has_wildcard() {
            return Err(MqttError::UnsupportedByServer);
        }

        if !self.server_config.subscription_identifiers_supported
            && options.subscription_identifier.is_some()
        {
            return Err(MqttError::UnsupportedByServer);
        }

        if !self.server_config.shared_subscription_supported && topic_filter.is_shared() {
            return Err(MqttError::UnsupportedByServer);
        }

        let subscribe_filter = SubscriptionFilter::new(topic_filter, options)
            .map_err(|NoLocalSharedSubscription| MqttError::IllegalNoLocalSharedSubscription)?;
        let subscribe_filters = [subscribe_filter].into();

        let Some(handle) = self.session.free_handle() else {
            info!("no free packet identifier");
            return Err(MqttError::SessionBuffer);
        };
        let pid = handle.packet_identifier;

        handle.outbound_sub().map_err(|_| {
            info!("maximum concurrent subscriptions reached");
            MqttError::SessionBuffer
        })?;

        let packet = SubscribePacket::<1, MAX_USER_PROPERTIES>::new(
            pid,
            options.subscription_identifier.map(Into::into),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
            subscribe_filters,
        )
        .expect("SUBSCRIBE with a single topic can not exceed VarByteInt::MAX_ENCODABLE");

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        debug!("sending SUBSCRIBE packet");

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(pid)
    }

    /// Unsubscribes from a single topic filter.
    ///
    /// The client keeps track of the packet identifier sent in the UNSUBSCRIBE packet.
    /// If no [`Event::Unsuback`] is received within a custom time,
    /// this method can be used to send the UNSUBSCRIBE packet again.
    ///
    /// # Returns:
    /// The packet identifier of the sent UNSUBSCRIBE packet.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::SessionBuffer`] if the buffer for outgoing UNSUBSCRIBE packet identifiers is full
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this UNSUBSCRIBE packet
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`UnsubscriptionOptions`]
    /// is greater than `MAX_USER_PROPERTIES`.
    pub async fn unsubscribe(
        &mut self,
        topic_filter: TopicFilter<'_>,
        options: &UnsubscriptionOptions<'_>,
    ) -> Result<PacketIdentifier, MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send UNSUBSCRIBE with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        let Some(handle) = self.session.free_handle() else {
            info!("no free packet identifier");
            return Err(MqttError::SessionBuffer);
        };
        let pid = handle.packet_identifier;

        handle.outbound_unsub().map_err(|_| {
            info!("maximum concurrent unsubscriptions reached");
            MqttError::SessionBuffer
        })?;

        let topic_filters = [topic_filter].into();
        let packet = UnsubscribePacket::<1, MAX_USER_PROPERTIES>::new(
            pid,
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
            topic_filters,
        )
        .expect("UNSUBSCRIBE with a single topic cannot exceed VarByteInt::MAX_ENCODABLE");

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        debug!("sending UNSUBSCRIBE packet");

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(pid)
    }

    /// Start the publication of a message. If [`QoS`] is greater than [`QoS::AtMostOnce`], the
    /// packet identifier is also kept track of by the client and further steps are necessary
    /// by the server and may be necessary by the client.
    ///
    /// Note:
    /// * The [`QoS`] should be less than or equal to the server's maximum [`QoS`].
    /// * The retain flag should only be set if the server supports retain.
    /// * A topic alias must be less than or equal to the server's maximum topic alias.
    ///
    /// The server support of these requirements can be checked via [`Client::server_config`].
    /// If a violation occurs, the client will not publish but prevent the protocol error
    /// and return an error.
    ///
    /// # Returns:
    /// - In case of [`QoS::AtMostOnce`]: [`None`]
    /// - In case of [`QoS::AtLeastOnce`] or [`QoS::ExactlyOnce`]: [`Some`] with the packet identifier
    ///   of the published packet. This value is required in case of a republication attempt.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::SendQuotaExceeded`] if the server's control flow limit is reached and sending
    ///   the PUBLISH would exceed the limit causing a protocol error
    /// * [`MqttError::SessionBuffer`] if the buffer for outgoing PUBLISH packet identifiers is full
    /// * [`MqttError::PacketMaximumLengthExceeded`] if the PUBLISH packet is too long to be encoded
    ///   with MQTT's [`VarByteInt`]
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this PUBLISH packet
    /// * [`MqttError::UnsupportedByServer`]
    ///   * if the quality of service level in the [`PublicationOptions`] is greater than the maximum
    ///     value specified in the server's CONNACK packet
    ///   * if the server specified in its CONNACK that retain is not available and a publication with
    ///     the retain flag set to true is attempted
    ///   * if a topic alias is used and its value is greater than the maximum value specified in the
    ///     server's CONNACK packet
    /// * [`MqttError::AllPacketIdentifiersUsed`] if there are no free packet identifiers in the pool
    ///   of outgoing publications, SUBSCRIBEs and UNSUBSCRIBEs
    /// * [`MqttError::ManualAckNotAllowed`] if the [`QoS`] is [`QoS::AtMostOnce`] or
    ///   [`QoS::AtLeastOnce`] and [`PublicationOptions::ack_mode`] is [`AckMode::Manual`]
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`PublicationOptions`]
    /// is greater than `MAX_USER_PROPERTIES`.
    pub async fn publish(
        &mut self,
        options: &PublicationOptions<'_>,
        message: Bytes<'_>,
    ) -> Result<Option<PacketIdentifier>, MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to publish {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        if (matches!(options.qos, QoS::AtMostOnce | QoS::AtLeastOnce)
            && options.ack_mode == AckMode::Manual)
        {
            return Err(MqttError::ManualAckNotAllowed);
        }

        if options.qos > self.server_config.maximum_qos {
            return Err(MqttError::UnsupportedByServer);
        }

        if !self.server_config.retain_supported && options.retain {
            return Err(MqttError::UnsupportedByServer);
        }

        if options
            .topic
            .alias()
            .map(NonZero::get)
            .is_some_and(|a| a > self.server_config.topic_alias_maximum)
        {
            return Err(MqttError::UnsupportedByServer);
        }

        let (identified_qos, handle) = if options.qos > QoS::AtMostOnce {
            if self.remaining_send_quota() == 0 {
                info!("server receive maximum reached");
                return Err(MqttError::SendQuotaExceeded);
            }

            let Some(handle) = self.session.free_handle() else {
                return Err(MqttError::AllPacketIdentifiersUsed);
            };

            match options.qos {
                QoS::AtMostOnce => unreachable!(),
                QoS::AtLeastOnce => (
                    IdentifiedQoS::AtLeastOnce(handle.packet_identifier),
                    Some(handle),
                ),
                QoS::ExactlyOnce => (
                    IdentifiedQoS::ExactlyOnce(handle.packet_identifier),
                    Some(handle),
                ),
            }
        } else {
            (IdentifiedQoS::AtMostOnce, None)
        };

        let packet = PublishPacket::<0, MAX_USER_PROPERTIES>::new(
            false,
            identified_qos,
            options.retain,
            options.topic.as_borrowed(),
            options.payload_format_indicator.map(Into::into),
            options.message_expiry_interval.map(Into::into),
            options.response_topic.as_ref().map(TopicName::as_borrowed),
            options
                .correlation_data
                .as_ref()
                .map(MqttBinary::as_borrowed),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
            options
                .content_type
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            message,
        )?;

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        if let Some(handle) = handle {
            // Treat the packet as sent before successfully sending. In case of a network error,
            // we have tracked the packet as in flight and can republish it.
            if let Err(e) = handle.outbound_publish(options.qos, options.ack_mode) {
                match e {
                    SmError::NoCapacity => return Err(MqttError::SessionBuffer),
                    SmError::PacketIdentifierUnused
                    | SmError::QoSMismatched
                    | SmError::HandshakeStateMismatched => unreachable!(),
                }
            }
        }

        match identified_qos.packet_identifier() {
            Some(pid) => debug!("sending PUBLISH packet with packet identifier {}", pid),
            None => debug!("sending PUBLISH packet"),
        }

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(identified_qos.packet_identifier())
    }

    /// Resends a PUBLISH packet with DUP flag set.
    ///
    /// This method must be called and must only be called after a reconnection with clean start set to 0,
    /// for every unacknowledged PUBLISH packet, as resending packets at any other time is a protocol error.
    /// (Compare [Message delivery retry](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901238), \[MQTT-4.4.0-1\]).
    ///
    /// Note:
    /// * Server-side constraints:
    ///   * The [`QoS`] should be less than or equal to the server's maximum [`QoS`].
    ///   * The retain flag should only be set if the server supports retain.
    ///   * A topic alias must be less than or equal to the server's maximum topic alias.
    /// * Client-side preconditions:
    ///   * The [`QoS`] must be [`QoS::AtLeastOnce`] or [`QoS::ExactlyOnce`] and must be the same as
    ///     that of the original publication.
    ///   * The packet identifier must have an in flight entry with the same [`QoS`] as the value
    ///     in the options parameter.
    ///   * If [`QoS`] is [`QoS::ExactlyOnce`], the in flight entry it must not already be awaiting
    ///     the PUBCOMP packet.
    ///   * The previous PUBLISH packet must have been sent in a different, previous network
    ///     connection.
    ///
    /// If a violation occurs, the client will not publish but prevent the protocol error
    /// and return an error. The server support of these requirements can be checked via
    /// [`Client::server_config`].
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::QoSMismatched`] if the [`QoS`] of this republish does not match the
    ///   [`QoS`] that this packet identifier was originally published with
    /// * [`MqttError::HandshakeStateMismatched`]
    ///   * if a PUBREC packet with this packet identifier has already been received and the server
    ///     has therefore already received the PUBLISH. Our next action is sending a PUBREL packet
    ///   * if the previous PUBLISH packet was sent in the same network connection
    /// * [`MqttError::PacketIdentifierNotInFlight`] if this packet identifier is not tracked in
    ///   an outgoing publication in the client's session
    /// * [`MqttError::PacketMaximumLengthExceeded`] if the PUBLISH packet is too long to be encoded
    ///   with MQTT's [`VarByteInt`]
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this PUBLISH packet
    /// * [`MqttError::UnsupportedByServer`]
    ///   * if the quality of service level in the [`PublicationOptions`] is greater than the maximum
    ///     value specified in the server's CONNACK packet
    ///   * if the server specified in its CONNACK that retain is not available and a publication with
    ///     the retain flag set to true is attempted
    ///   * if a topic alias is used and its value is greater than the maximum value specified in the
    ///     server's CONNACK packet
    ///
    /// # Panics
    ///
    /// This function may panic if the [`QoS`] in the `options` is [`QoS::AtMostOnce`].
    /// This function panics if the length of the `user_properties` slice in the [`PublicationOptions`]
    /// is greater than `MAX_USER_PROPERTIES`.
    pub async fn republish(
        &mut self,
        packet_identifier: PacketIdentifier,
        options: &PublicationOptions<'_>,
        message: Bytes<'_>,
    ) -> Result<(), MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to publish {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        assert_ne!(
            options.qos,
            QoS::AtMostOnce,
            "QoS 0 packets cannot be republished"
        );

        if options.qos > self.server_config.maximum_qos {
            return Err(MqttError::UnsupportedByServer);
        }

        if !self.server_config.retain_supported && options.retain {
            return Err(MqttError::UnsupportedByServer);
        }

        if options
            .topic
            .alias()
            .map(NonZero::get)
            .is_some_and(|a| a > self.server_config.topic_alias_maximum)
        {
            return Err(MqttError::UnsupportedByServer);
        }

        let identified_qos = match options.qos {
            QoS::AtMostOnce => unreachable!(),
            QoS::AtLeastOnce => IdentifiedQoS::AtLeastOnce(packet_identifier),
            QoS::ExactlyOnce => IdentifiedQoS::ExactlyOnce(packet_identifier),
        };

        let packet = PublishPacket::<0, MAX_USER_PROPERTIES>::new(
            true,
            identified_qos,
            options.retain,
            options.topic.as_borrowed(),
            options.payload_format_indicator.map(Into::into),
            options.message_expiry_interval.map(Into::into),
            options.response_topic.as_ref().map(TopicName::as_borrowed),
            options
                .correlation_data
                .as_ref()
                .map(MqttBinary::as_borrowed),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
            options
                .content_type
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            message,
        )?;

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        if let Err(e) = self.session.outbound_republish(identified_qos) {
            match e {
                SmError::NoCapacity => {
                    unreachable!("a republish can not fail due to missing capacity")
                }
                SmError::PacketIdentifierUnused => {
                    return Err(MqttError::PacketIdentifierNotInFlight);
                }
                SmError::QoSMismatched => {
                    return Err(MqttError::QoSMismatched);
                }
                SmError::HandshakeStateMismatched => {
                    return Err(MqttError::HandshakeStateMismatched);
                }
            }
        }

        debug!(
            "resending PUBLISH packet with packet identifier {}",
            packet_identifier
        );

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Resends all pending PUBREL packets that are belong to publication flows started  with the default
    /// [`AckMode::Automatic`]. PUBREL packets associated with [`AckMode::Manual`] flows must be resent
    /// manually on a per-packet basis with [`Client::manual_release`]. To include a reason string and/or
    /// user properties in such a retransmitted PUBREL packet, [`Client::manual_release`] can be used for
    /// individual packets and this method for any remaining, not manually retransmitted PUBREL packets.
    ///
    /// This method should only be called once after a reconnection with clean start set to 0, as resending
    /// packets at any other time is a protocol error and is prevented by the client anyway.
    /// (Compare [Message delivery retry](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901238), \[MQTT-4.4.0-1\]).
    ///
    /// This method assumes that the server's receive maximum after the reconnection is great enough
    /// to handle as many publication flows as dragged between the two connections.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    pub async fn rerelease(&mut self) -> Result<(), MqttError<'c, 0>> {
        let Some(mut handle) = self.session.outbound_iter() else {
            return Ok(());
        };

        loop {
            if handle.state == LocalPublishState::DueRel(AckMode::Automatic) {
                handle.outbound_pubrel().unwrap();

                let pubrel =
                    PubrelPacket::<0>::minimal(handle.packet_identifier(), ReasonCode::Success);

                debug!("sending PUBREL packet {}", pubrel.packet_identifier);

                // Don't check whether length exceeds servers maximum packet size because we don't
                // add properties to automatically retransmitted PUBREL packets -> length is always
                // minimal at 6 bytes. The server really shouldn't reject this.
                self.raw.send(&pubrel).await?;
            }
            if let Some(next) = handle.next() {
                handle = next;
            } else {
                break;
            }
        }

        self.raw.flush().await?;

        Ok(())
    }

    /// Sends a PUBACK packet responding to an incoming [`QoS::AtLeastOnce`] PUBLISH packet, that
    /// was marked with [`AckMode::Manual`].
    ///
    /// The reason code must be one of [`ReasonCode::Success`], [`ReasonCode::UnspecifiedError`],
    /// [`ReasonCode::ImplementationSpecificError`], [`ReasonCode::NotAuthorized`],
    /// [`ReasonCode::TopicNameInvalid`], [`ReasonCode::PacketIdentifierInUse`],
    /// [`ReasonCode::QuotaExceeded`] or [`ReasonCode::PayloadFormatInvalid`]
    /// (Compare [PUBACK Reason Code](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901124), \[MQTT-3.4.2-1\]).
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::IllegalReasonCode`] if the selected reason code is not allowed
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would
    ///   be exceeded by sending this PUBACK packet
    /// * [`MqttError::QoSMismatched`] if this packet identifier is actually part of an incoming
    ///   [`QoS::ExactlyOnce`] publication
    /// * [`MqttError::HandshakeStateMismatched`] if a reconnection occured and this
    ///   the server has not resent the PUBLISH in this network connection.
    /// * [`MqttError::PacketIdentifierNotInFlight`] if this packet identifier is not tracked in
    ///   an incoming publication in the client's session
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`AckOptions`] is
    /// greater than `MAX_USER_PROPERTIES`.
    pub async fn manual_acknowledge(
        &mut self,
        packet_identifier: PacketIdentifier,
        reason_code: ReasonCode,
        options: &AckOptions<'_>,
    ) -> Result<(), MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send PUBACK with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        // Not allowed:
        // ReasonCode::NoMatchingSubscribers - only sent by the server
        if !matches!(
            reason_code,
            ReasonCode::Success
                | ReasonCode::UnspecifiedError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::NotAuthorized
                | ReasonCode::TopicNameInvalid
                | ReasonCode::PacketIdentifierInUse
                | ReasonCode::QuotaExceeded
                | ReasonCode::PayloadFormatInvalid
        ) {
            return Err(MqttError::IllegalReasonCode);
        }

        let packet = PubackPacket::<MAX_USER_PROPERTIES>::new(
            packet_identifier,
            reason_code,
            options
                .reason_string
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
        );

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        self.session
            .outbound_puback(packet_identifier)
            .map_err(|e| match e {
                SmError::NoCapacity => unreachable!(),
                SmError::PacketIdentifierUnused => MqttError::PacketIdentifierNotInFlight,
                SmError::QoSMismatched => MqttError::QoSMismatched,
                SmError::HandshakeStateMismatched => MqttError::HandshakeStateMismatched,
            })?;

        debug!("sending PUBACK packet {}", packet.packet_identifier);

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Sends a PUBREC packet responding to an incoming [`QoS::ExactlyOnce`] PUBLISH packet, that
    /// was marked with [`AckMode::Manual`].
    ///
    /// The reason code must be one of [`ReasonCode::Success`], [`ReasonCode::UnspecifiedError`],
    /// [`ReasonCode::ImplementationSpecificError`], [`ReasonCode::NotAuthorized`],
    /// [`ReasonCode::TopicNameInvalid`], [`ReasonCode::PacketIdentifierInUse`],
    /// [`ReasonCode::QuotaExceeded`] or [`ReasonCode::PayloadFormatInvalid`]
    /// (Compare [PUBREC Reason Code](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901134), \[MQTT-3.5.2-1\]).
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::IllegalReasonCode`] if the selected reason code is not allowed
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would
    ///   be exceeded by sending this PUBREC packet
    /// * [`MqttError::QoSMismatched`] if this packet identifier is actually part of an incoming
    ///   [`QoS::AtLeastOnce`] publication
    /// * [`MqttError::HandshakeStateMismatched`]
    ///   * if a PUBREC has already been sent for this packet identifier in this network connection
    ///   * if a reconnection occured and the server has not resent the PUBLISH in this network
    ///     connection
    /// * [`MqttError::PacketIdentifierNotInFlight`] if this packet identifier is not tracked in
    ///   an incoming publication in the client's session
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`AckOptions`] is
    /// greater than `MAX_USER_PROPERTIES`.
    pub async fn manual_receive(
        &mut self,
        packet_identifier: PacketIdentifier,
        reason_code: ReasonCode,
        options: &AckOptions<'_>,
    ) -> Result<(), MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send PUBREC with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        // Not allowed:
        // ReasonCode::NoMatchingSubscribers - only sent by the server
        if !matches!(
            reason_code,
            ReasonCode::Success
                | ReasonCode::UnspecifiedError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::NotAuthorized
                | ReasonCode::TopicNameInvalid
                | ReasonCode::PacketIdentifierInUse
                | ReasonCode::QuotaExceeded
                | ReasonCode::PayloadFormatInvalid
        ) {
            return Err(MqttError::IllegalReasonCode);
        }

        let packet = PubrecPacket::<MAX_USER_PROPERTIES>::new(
            packet_identifier,
            reason_code,
            options
                .reason_string
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
        );

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        self.session
            .outbound_pubrec(packet_identifier, reason_code)
            .map_err(|e| match e {
                SmError::NoCapacity => unreachable!(),
                SmError::PacketIdentifierUnused => MqttError::PacketIdentifierNotInFlight,
                SmError::QoSMismatched => MqttError::QoSMismatched,
                SmError::HandshakeStateMismatched => MqttError::HandshakeStateMismatched,
            })?;

        debug!("sending PUBREC packet {}", packet.packet_identifier);

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Sends a PUBREL packet responding to an incoming PUBREC packet belonging to an
    /// outgoing publication flow configured with [`AckMode::Manual`].
    ///
    /// The reason code is implicitely [`ReasonCode::Success`] as the only other allowed
    /// reason code is erroneous and packet identifier related, which is something the
    /// client handles.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would
    ///   be exceeded by sending this PUBREL packet
    /// * [`MqttError::QoSMismatched`] if this packet identifier is actually part of an outgoing
    ///   [`QoS::AtLeastOnce`] publication
    /// * [`MqttError::HandshakeStateMismatched`]
    ///   * if the client hasn't yet received a PUBREC from the server for this packet identifier
    ///   * if a PUBREL has already been sent for this packet identifier in this network connection
    /// * [`MqttError::PacketIdentifierNotInFlight`] if this packet identifier is not tracked in
    ///   an incoming publication in the client's session
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`AckOptions`] is
    /// greater than `MAX_USER_PROPERTIES`.
    pub async fn manual_release(
        &mut self,
        packet_identifier: PacketIdentifier,
        options: &AckOptions<'_>,
    ) -> Result<(), MqttError<'c, 0>> {
        const REASON_CODE: ReasonCode = ReasonCode::Success;

        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send PUBREL with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        let packet = PubrelPacket::<MAX_USER_PROPERTIES>::new(
            packet_identifier,
            REASON_CODE,
            options
                .reason_string
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
        );

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        self.session
            .outbound_pubrel(packet_identifier)
            .map_err(|e| match e {
                SmError::NoCapacity => unreachable!(),
                SmError::PacketIdentifierUnused => MqttError::PacketIdentifierNotInFlight,
                SmError::QoSMismatched => MqttError::QoSMismatched,
                SmError::HandshakeStateMismatched => MqttError::HandshakeStateMismatched,
            })?;

        debug!("sending PUBREL packet {}", packet.packet_identifier);

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Sends a PUBCOMP packet responding to an incoming PUBREL packet belonging to an
    /// incoming publication flow, that was marked with [`AckMode::Manual`].
    ///
    /// The reason code is implicitely [`ReasonCode::Success`] as the only other allowed
    /// reason code is erroneous and packet identifier related, which is something the
    /// client handles.
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would
    ///   be exceeded by sending this PUBCOMP packet
    /// * [`MqttError::QoSMismatched`] if this packet identifier is actually part of an outgoing
    ///   [`QoS::AtLeastOnce`] publication
    /// * [`MqttError::HandshakeStateMismatched`] if the client hasn't yet received a PUBREL from
    ///   the server for this packet identifier in this network connection
    /// * [`MqttError::PacketIdentifierNotInFlight`] if this packet identifier is not tracked in
    ///   an incoming publication in the client's session
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`AckOptions`] is
    /// greater than `MAX_USER_PROPERTIES`.
    pub async fn manual_complete(
        &mut self,
        packet_identifier: PacketIdentifier,
        options: &AckOptions<'_>,
    ) -> Result<(), MqttError<'c, 0>> {
        const REASON_CODE: ReasonCode = ReasonCode::Success;

        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send PUBCOMP with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        let packet = PubcompPacket::<MAX_USER_PROPERTIES>::new(
            packet_identifier,
            REASON_CODE,
            options
                .reason_string
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
        );

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        self.session
            .outbound_pubcomp(packet_identifier)
            .map_err(|e| match e {
                SmError::NoCapacity => unreachable!(),
                SmError::PacketIdentifierUnused => MqttError::PacketIdentifierNotInFlight,
                SmError::QoSMismatched => MqttError::QoSMismatched,
                SmError::HandshakeStateMismatched => MqttError::HandshakeStateMismatched,
            })?;

        debug!("sending PUBCOMP packet {}", packet.packet_identifier);

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Disconnects from the server after an error occured in a situation-aware way by either:
    /// - dropping the connection
    /// - sending a DISCONNECT with the deposited reason code and dropping the connection.
    ///
    /// After an MQTT communication fails, usually either the client or the server closes the connection.
    ///
    /// This is not cancel-safe but you can set a timeout if reconnecting later anyway or you don't reuse the client.
    ///
    /// # Panics
    ///
    /// This function may panic if the client has not returned an unrecoverable error before.
    #[inline]
    pub async fn abort(&mut self) {
        match self.raw.abort().await {
            Ok(()) => info!("connection aborted"),
            Err(e) => warn!("connection abort failed: {:?}", e),
        }
    }

    /// Disconnects gracefully from the server by sending a DISCONNECT packet.
    ///
    /// The reason code must be one of [`ReasonCode::Success`], [`ReasonCode::DisconnectWithWillMessage`],
    /// [`ReasonCode::UnspecifiedError`], [`ReasonCode::MalformedPacket`], [`ReasonCode::ProtocolError`],
    /// [`ReasonCode::ImplementationSpecificError`], [`ReasonCode::TopicNameInvalid`],
    /// [`ReasonCode::ReceiveMaximumExceeded`], [`ReasonCode::TopicAliasInvalid`], [`ReasonCode::PacketTooLarge`],
    /// [`ReasonCode::MessageRateTooHigh`], [`ReasonCode::QuotaExceeded`], [`ReasonCode::AdministrativeAction`] or
    /// [`ReasonCode::PayloadFormatInvalid`]
    /// (Compare [Disconnect Reason Code](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901208), \[MQTT-3.14.2-1\]).
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::IllegalReasonCode`] if the selected reason code is not allowed
    /// * [`MqttError::IllegalDisconnectSessionExpiryInterval`] if the session expiry interval in the
    ///   CONNECT packet was zero and the session expiry interval in the [`DisconnectOptions`] is [`Some`]
    ///   and not [`SessionExpiryInterval::EndOnDisconnect`].
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this DISCONNECT packet
    ///
    /// # Panics
    ///
    /// This function panics if the length of the `user_properties` slice in the [`DisconnectOptions`]
    /// is greater than `MAX_USER_PROPERTIES`.
    pub async fn disconnect(
        &mut self,
        options: &DisconnectOptions<'_>,
    ) -> Result<(), MqttError<'c, 0>> {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "attempted to send DISCONNECT with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        // Not allowed:
        // ReasonCode::NotAuthorized - only sent by the server
        // ReasonCode::ServerBusy - only sent by the server
        // ReasonCode::ServerShuttingDown - only sent by the server
        // ReasonCode::KeepAliveTimeout - only sent by the server
        // ReasonCode::SessionTakenOver - only sent by the server
        // ReasonCode::TopicFilterInvalid - only sent by the server
        // ReasonCode::RetainNotSupported - only sent by the server
        // ReasonCode::QoSNotSupported - only sent by the server
        // ReasonCode::UseAnotherServer - only sent by the server
        // ReasonCode::ServerMoved - only sent by the server
        // ReasonCode::SharedSubscriptionsNotSupported - only sent by the server
        // ReasonCode::ConnectionRateExceeded - only sent by the server
        // ReasonCode::MaximumConnectTime - only sent by the server
        // ReasonCode::SubscriptionIdentifiersNotSupported - only sent by the server
        // ReasonCode::WildcardSubscriptionsNotSupported - only sent by the server
        if !matches!(
            options.reason_code,
            ReasonCode::Success
                | ReasonCode::DisconnectWithWillMessage
                | ReasonCode::UnspecifiedError
                | ReasonCode::MalformedPacket
                | ReasonCode::ProtocolError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::TopicNameInvalid
                | ReasonCode::ReceiveMaximumExceeded
                | ReasonCode::TopicAliasInvalid
                | ReasonCode::PacketTooLarge
                | ReasonCode::MessageRateTooHigh
                | ReasonCode::QuotaExceeded
                | ReasonCode::AdministrativeAction
                | ReasonCode::PayloadFormatInvalid
        ) {
            return Err(MqttError::IllegalReasonCode);
        }

        let connect_session_expiry_interval_was_zero =
            self.client_config.session_expiry_interval == SessionExpiryInterval::EndOnDisconnect;
        let disconnect_session_expiry_interval_is_non_zero = options
            .session_expiry_interval
            .is_some_and(|s| s != SessionExpiryInterval::EndOnDisconnect);

        if connect_session_expiry_interval_was_zero
            && disconnect_session_expiry_interval_is_non_zero
        {
            return Err(MqttError::IllegalDisconnectSessionExpiryInterval);
        }

        let packet = DisconnectPacket::<MAX_USER_PROPERTIES>::new(
            options.reason_code,
            options.session_expiry_interval,
            options
                .reason_string
                .as_ref()
                .map(MqttString::as_borrowed)
                .map(Into::into),
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
        );

        if self.server_config.maximum_packet_size.as_u32() < packet.encoded_len() as u32 {
            return Err(MqttError::ServerMaximumPacketSizeExceeded);
        }

        debug!("sending DISCONNECT packet");

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        // Terminates (closes) the connection by dropping it
        self.raw.close_with(None);

        info!("disconnected from server");

        Ok(())
    }

    /// Combines [`Self::poll_header`] and [`Self::poll_body`].
    ///
    /// Polls the network for a full packet. Not cancel-safe.
    ///
    /// # Preconditions:
    /// - The last MQTT packet was received completely
    /// - The client did not return a non-recoverable Error before
    ///
    /// # Returns:
    /// MQTT Events. Their further meaning is documented in [`Event`].
    ///
    /// # Errors
    ///
    /// Returns the errors that [`Client::poll_header`] and [`Client::poll_body`] return.
    /// For further information view their docs.
    pub async fn poll(
        &mut self,
    ) -> Result<
        Event<'c, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>,
        MqttError<'c, MAX_USER_PROPERTIES>,
    > {
        let header = self.poll_header().await.map_err(MqttError::inflate)?;
        self.poll_body(header).await
    }

    /// Polls the network for a fixed header in a cancel-safe way.
    ///
    /// If a fixed header is received, the first 4 bits (packet type) are checked for correctness.
    ///
    /// # Preconditions:
    /// - The last MQTT packet was received completely
    /// - The client did not return a non-recoverable Error before
    ///
    /// # Returns:
    /// The received fixed header with a valid packet type. It can be used to call [`Self::poll_body`].
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::Server`] if:
    ///   * the server sends a malformed packet header
    ///   * the packet following this header exceeds the client's maximum packet size
    pub async fn poll_header(&mut self) -> Result<FixedHeader, MqttError<'c, 0>> {
        let header = self.raw.recv_header().await?;

        if let Ok(p) = header.packet_type() {
            debug!(
                "received {:?} packet header (remaining length: {})",
                p,
                header.remaining_len.value()
            );
        } else {
            error!("received invalid header {:?}", header);
            self.raw.close_with(Some(ReasonCode::MalformedPacket));
            return Err(MqttError::Server);
        }

        if header.remaining_len.value() > self.client_config.maximum_accepted_remaining_length {
            error!(
                "received a packet exceeding maximum packet size, remaining length={:?}",
                header.remaining_len.value()
            );
            self.raw.close_with(Some(ReasonCode::PacketTooLarge));
            return Err(MqttError::Server);
        }

        Ok(header)
    }

    /// Polls the network for the variable header and payload of a packet. Not cancel-safe.
    ///
    /// # Preconditions:
    /// - The [`FixedHeader`] argument was received from the network right before.
    /// - The client did not return a non-recoverable [`MqttError`] before
    ///
    /// # Returns:
    /// MQTT Events for regular communication. Their further meaning is documented in [`Event`].
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::Alloc`] if the underlying [`BufferProvider`] returned an error
    /// * [`MqttError::Server`] if:
    ///   * the server sends a malformed packet
    ///   * the server causes a protocol error
    ///   * the packet following this header exceeds the client's maximum packet size
    ///   * the server sends a PUBLISH packet with an invalid topic alias
    ///   * the server exceeded the client's receive maximum with a new [`QoS::ExactlyOnce`]
    ///     PUBLISH
    ///   * the server sends a PUBACK/PUBREC/PUBREL/PUBCOMP packet which mismatches what
    ///     the client expects for this packet identifier from its session state
    ///   * the fixed header has the packet type CONNECT/SUBSCRIBE/UNSUBSCRIBE/PINGREQ
    /// * [`MqttError::Disconnect`] if a DISCONNECT packet is received
    /// * [`MqttError::AuthPacketReceived`] if the fixed header has the packet type AUTH
    pub async fn poll_body(
        &mut self,
        header: FixedHeader,
    ) -> Result<
        Event<'c, MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>,
        MqttError<'c, MAX_USER_PROPERTIES>,
    > {
        let event = match header.packet_type()? {
            PacketType::Pingresp => {
                self.raw.recv_body::<PingrespPacket>(&header).await?;
                Event::Pingresp
            }
            PacketType::Suback => {
                // We only send SUBSCRIBE packets with exactly 1 topic
                // -> Packets with more than 1 reason code are currently rejected by the RxPacket::receive implementation
                //    with RxError::Protocol error. This is correct as long as we only send SUBSCRIBE packets with 1 topic.
                let suback = self
                    .raw
                    .recv_body::<SubackPacket<1, MAX_USER_PROPERTIES>>(&header)
                    .await?;

                if !self.client_config.request_problem_information
                    && (suback.reason_string.is_some() || !suback.user_properties.is_empty())
                {
                    error!(
                        "server sent reason string or user properties when request problem information was false"
                    );
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                }

                let pid = suback.packet_identifier;

                if let Some(h) = self.session.sub_handle(pid) {
                    h.remove();

                    // We only send SUBSCRIBE packets with exactly 1 topic
                    let [r] = suback.reason_codes.as_slice() else {
                        error!("received mismatched SUBACK");
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    };

                    Event::Suback(Suback {
                        packet_identifier: pid,
                        reason_string: suback.reason_string.map(Property::into_inner),
                        user_properties: suback
                            .user_properties
                            .into_iter()
                            .map(Property::into_inner)
                            .collect(),
                        reason_code: *r,
                    })
                } else {
                    debug!("packet identifier {} in SUBACK not in use", pid);
                    Event::Ignored
                }
            }
            PacketType::Unsuback => {
                // We only send UNSUBSCRIBE packets with exactly 1 topic
                // -> Packets with more than 1 reason code are currently rejected by the RxPacket::receive implementation
                //    with RxError::Protocol error. This is correct as long as we only send UNSUBSCRIBE packets with 1 topic.
                let unsuback = self
                    .raw
                    .recv_body::<UnsubackPacket<1, MAX_USER_PROPERTIES>>(&header)
                    .await?;

                if !self.client_config.request_problem_information
                    && (unsuback.reason_string.is_some() || !unsuback.user_properties.is_empty())
                {
                    error!(
                        "server sent reason string or user properties when request problem information was false"
                    );
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                }

                let pid = unsuback.packet_identifier;

                if let Some(h) = self.session.unsub_handle(pid) {
                    h.remove();

                    // We only send UNSUBSCRIBE packets with exactly 1 topic
                    let [r] = unsuback.reason_codes.as_slice() else {
                        error!("received mismatched UNSUBACK");
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    };

                    Event::Unsuback(Suback {
                        packet_identifier: pid,
                        reason_string: unsuback.reason_string.map(Property::into_inner),
                        user_properties: unsuback
                            .user_properties
                            .into_iter()
                            .map(Property::into_inner)
                            .collect(),
                        reason_code: *r,
                    })
                } else {
                    debug!("packet identifier {} in UNSUBACK not in use", pid);
                    Event::Ignored
                }
            }
            PacketType::Publish => {
                let publish = self
                    .raw
                    .recv_body::<PublishPacket<MAX_SUBSCRIPTION_IDENTIFIERS, MAX_USER_PROPERTIES>>(
                        &header,
                    )
                    .await?;

                // Our topic alias maximum is always 0, the moment we receive a topic alias, this is an error.
                let TopicReference::Name(topic) = publish.topic else {
                    error!("received disallowed topic alias");
                    self.raw.close_with(Some(ReasonCode::TopicAliasInvalid));
                    return Err(MqttError::Server);
                };

                let publish = Publish {
                    ack_mode: AckMode::default(),
                    dup: publish.dup,
                    identified_qos: publish.identified_qos,
                    retain: publish.retain,
                    topic,
                    payload_format_indicator: publish
                        .payload_format_indicator
                        .map(Property::into_inner),
                    message_expiry_interval: publish
                        .message_expiry_interval
                        .map(Property::into_inner),
                    response_topic: publish.response_topic.map(Property::into_inner),
                    correlation_data: publish.correlation_data.map(Property::into_inner),
                    user_properties: publish
                        .user_properties
                        .into_iter()
                        .map(Property::into_inner)
                        .collect(),
                    subscription_identifiers: publish
                        .subscription_identifiers
                        .into_iter()
                        .map(Property::into_inner)
                        .collect(),
                    content_type: publish.content_type.map(Property::into_inner),
                    message: publish.message,
                };

                let ack_mode = if (self.manual_ack_when)(&publish) {
                    AckMode::Manual
                } else {
                    AckMode::Automatic
                };

                let publish = Publish {
                    ack_mode,
                    ..publish
                };

                let (action, event) = self
                    .session
                    .inbound_publish(publish.identified_qos, publish.ack_mode);

                match action {
                    Response::Release(_) | Response::Complete(_) => unreachable!(),

                    Response::None => {}
                    Response::Acknowledge(reason_code) => {
                        let puback = PubackPacket::<0>::minimal(
                            publish.identified_qos.packet_identifier().unwrap(),
                            reason_code,
                        );
                        debug!("sending PUBACK packet {}", puback.packet_identifier);

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to automatically sent PUBACK packets -> length is always minimal
                        // at 6 bytes. The server really shouldn't reject this.
                        self.raw.send(&puback).await?;
                        self.raw.flush().await?;
                    }
                    Response::Receive(reason_code) => {
                        let pubrec = PubrecPacket::<0>::minimal(
                            publish.identified_qos.packet_identifier().unwrap(),
                            reason_code,
                        );
                        debug!("sending PUBREC packet {}", pubrec.packet_identifier);

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to automatically sent PUBREC packets -> length is always minimal
                        // at 6 bytes. The server really shouldn't reject this.
                        self.raw.send(&pubrec).await?;
                        self.raw.flush().await?;
                    }
                    Response::Disconnect(reason_code) => {
                        error!("invalid PUBLISH packet rejected by state machine");
                        self.raw.close_with(Some(reason_code));
                    }
                }

                match event {
                    SmEvent::Aborted
                    | SmEvent::Rejected
                    | SmEvent::Acknowledged
                    | SmEvent::Received(_)
                    | SmEvent::Released(_)
                    | SmEvent::Completed => unreachable!(),

                    SmEvent::Publish => Event::Publish(publish),
                    SmEvent::Duplicate(ack_mode) => {
                        let publish = Publish {
                            ack_mode,
                            ..publish
                        };
                        Event::Duplicate(publish)
                    }
                    SmEvent::Ignored => Event::Ignored,
                    SmEvent::ServerError => return Err(MqttError::Server),
                }
            }
            PacketType::Puback => {
                let puback = self
                    .raw
                    .recv_body::<PubackPacket<MAX_USER_PROPERTIES>>(&header)
                    .await?;

                if !self.client_config.request_problem_information
                    && (puback.reason_string.is_some() || !puback.user_properties.is_empty())
                {
                    error!(
                        "server sent reason string or user properties when request problem information was false"
                    );
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                }

                let (action, event) = self
                    .session
                    .inbound_puback(puback.packet_identifier, puback.reason_code);

                match action {
                    Response::Acknowledge(_)
                    | Response::Receive(_)
                    | Response::Release(_)
                    | Response::Complete(_) => unreachable!(),

                    Response::None => {}
                    Response::Disconnect(reason_code) => {
                        error!("invalid PUBACK packet rejected by state machine");
                        self.raw.close_with(Some(reason_code));
                    }
                }

                match event {
                    SmEvent::Publish
                    | SmEvent::Duplicate(_)
                    | SmEvent::Aborted
                    | SmEvent::Received(_)
                    | SmEvent::Released(_)
                    | SmEvent::Completed => unreachable!(),

                    SmEvent::Ignored => Event::Ignored,
                    SmEvent::Rejected => Event::PublishRejected(Pubrej::from(puback)),
                    SmEvent::Acknowledged => {
                        Event::PublishAcknowledged(Puback::new(puback, AckMode::default()))
                    }
                    SmEvent::ServerError => return Err(MqttError::Server),
                }
            }
            PacketType::Pubrec => {
                let pubrec = self
                    .raw
                    .recv_body::<PubrecPacket<MAX_USER_PROPERTIES>>(&header)
                    .await?;

                if !self.client_config.request_problem_information
                    && (pubrec.reason_string.is_some() || !pubrec.user_properties.is_empty())
                {
                    error!(
                        "server sent reason string or user properties when request problem information was false"
                    );
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                }

                let (action, event) = self
                    .session
                    .inbound_pubrec(pubrec.packet_identifier, pubrec.reason_code);

                match action {
                    Response::Acknowledge(_) | Response::Receive(_) | Response::Complete(_) => {
                        unreachable!()
                    }

                    Response::None => {}
                    Response::Release(reason_code) => {
                        let pubrel =
                            PubrelPacket::<0>::minimal(pubrec.packet_identifier, reason_code);

                        debug!("sending PUBREL packet {}", pubrel.packet_identifier);

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to automatically sent PUBREL packets -> length is always minimal
                        // at 6 bytes. The server really shouldn't reject this.
                        self.raw.send(&pubrel).await?;
                        self.raw.flush().await?;
                    }
                    Response::Disconnect(reason_code) => {
                        error!("invalid PUBREC packet rejected by state machine");
                        self.raw.close_with(Some(reason_code));
                    }
                }

                match event {
                    SmEvent::Publish
                    | SmEvent::Duplicate(_)
                    | SmEvent::Aborted
                    | SmEvent::Acknowledged
                    | SmEvent::Released(_)
                    | SmEvent::Completed => unreachable!(),

                    SmEvent::Ignored => Event::Ignored,
                    SmEvent::Rejected => Event::PublishRejected(Pubrej::from(pubrec)),
                    SmEvent::Received(mode) => Event::PublishReceived(Puback::new(pubrec, mode)),
                    SmEvent::ServerError => return Err(MqttError::Server),
                }
            }
            PacketType::Pubrel => {
                let pubrel = self
                    .raw
                    .recv_body::<PubrelPacket<MAX_USER_PROPERTIES>>(&header)
                    .await?;

                if !self.client_config.request_problem_information
                    && (pubrel.reason_string.is_some() || !pubrel.user_properties.is_empty())
                {
                    error!(
                        "server sent reason string or user properties when request problem information was false"
                    );
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                }

                let (action, event) = self
                    .session
                    .inbound_pubrel(pubrel.packet_identifier, pubrel.reason_code);

                match action {
                    Response::Acknowledge(_) | Response::Receive(_) | Response::Release(_) => {
                        unreachable!()
                    }

                    Response::None => {}
                    Response::Complete(reason_code) => {
                        let pubcomp =
                            PubcompPacket::<0>::minimal(pubrel.packet_identifier, reason_code);

                        debug!("sending PUBCOMP packet {}", pubcomp.packet_identifier);

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to automatically sent PUBCOMP packets -> length is always minimal
                        // at 6 bytes. The server really shouldn't reject this.
                        self.raw.send(&pubcomp).await?;
                        self.raw.flush().await?;
                    }
                    Response::Disconnect(reason_code) => {
                        error!("invalid PUBREL packet rejected by state machine");
                        self.raw.close_with(Some(reason_code));
                    }
                }

                match event {
                    SmEvent::Publish
                    | SmEvent::Duplicate(_)
                    | SmEvent::Rejected
                    | SmEvent::Acknowledged
                    | SmEvent::Received(_)
                    | SmEvent::Completed => unreachable!(),

                    SmEvent::Ignored => Event::Ignored,
                    SmEvent::Aborted => Event::PublishAborted(Pubrej::from(pubrel)),
                    SmEvent::Released(mode) => Event::PublishReleased(Puback::new(pubrel, mode)),
                    SmEvent::ServerError => return Err(MqttError::Server),
                }
            }
            PacketType::Pubcomp => {
                let pubcomp = self
                    .raw
                    .recv_body::<PubcompPacket<MAX_USER_PROPERTIES>>(&header)
                    .await?;

                if !self.client_config.request_problem_information
                    && (pubcomp.reason_string.is_some() || !pubcomp.user_properties.is_empty())
                {
                    error!(
                        "server sent reason string or user properties when request problem information was false"
                    );
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    return Err(MqttError::Server);
                }

                let (action, event) = self
                    .session
                    .inbound_pubcomp(pubcomp.packet_identifier, pubcomp.reason_code);

                match action {
                    Response::Acknowledge(_)
                    | Response::Receive(_)
                    | Response::Release(_)
                    | Response::Complete(_) => unreachable!(),

                    Response::None => {}
                    Response::Disconnect(reason_code) => {
                        error!("invalid PUBCOMP packet rejected by state machine");
                        self.raw.close_with(Some(reason_code));
                    }
                }

                match event {
                    SmEvent::Publish
                    | SmEvent::Duplicate(_)
                    | SmEvent::Aborted
                    | SmEvent::Rejected
                    | SmEvent::Acknowledged
                    | SmEvent::Received(_)
                    | SmEvent::Released(_) => unreachable!(),

                    SmEvent::Ignored => Event::Ignored,
                    SmEvent::Completed => {
                        Event::PublishComplete(Puback::new(pubcomp, AckMode::default()))
                    }
                    SmEvent::ServerError => return Err(MqttError::Server),
                }
            }
            PacketType::Disconnect => {
                let disconnect = self
                    .raw
                    .recv_body::<DisconnectPacket<MAX_USER_PROPERTIES>>(&header)
                    .await?;

                // The server initiated the disconnect. We must close the transport on our side
                // as well so that subsequent error handling (e.g. `abort`) sees a non-Ok network state.
                self.raw.close_with(None);

                return Err(MqttError::Disconnect {
                    reason: disconnect.reason_code,
                    reason_string: disconnect.reason_string.map(Property::into_inner),
                    user_properties: disconnect
                        .user_properties
                        .into_iter()
                        .map(Property::into_inner)
                        .collect(),
                    server_reference: disconnect.server_reference.map(Property::into_inner),
                });
            }
            t @ (PacketType::Connect
            | PacketType::Subscribe
            | PacketType::Unsubscribe
            | PacketType::Pingreq) => {
                error!(
                    "received a packet that the server is not allowed to send: {:?}",
                    t
                );

                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::Server);
            }
            PacketType::Connack => {
                error!("received unexpected CONNACK packet");

                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::Server);
            }
            PacketType::Auth => {
                error!("received unexpected AUTH packet");

                // Receiving a AUTH packet is currently always a protocol error because we never send
                // an Authentication Method property in the CONNECT packet.
                // <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901217>
                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::AuthPacketReceived);
            }
        };

        Ok(event)
    }
}
