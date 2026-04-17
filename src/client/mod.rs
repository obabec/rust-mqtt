//! Implements full client functionality with session and configuration handling and Quality of Service flows.

use core::num::NonZero;

use heapless::Vec;

use crate::{
    buffer::BufferProvider,
    bytes::Bytes,
    client::{
        event::{Event, Puback, Publish, Pubrej, Suback},
        info::ConnectInfo,
        options::{
            ConnectOptions, DisconnectOptions, PublicationOptions, SubscriptionOptions,
            TopicReference, UnsubscriptionOptions,
        },
        raw::Raw,
    },
    config::{ClientConfig, MaximumPacketSize, ServerConfig, SessionExpiryInterval, SharedConfig},
    fmt::{assert, const_assert, debug, error, info, panic, trace, unreachable, warn},
    header::{FixedHeader, PacketType},
    io::Transport,
    packet::{Packet, TxPacket},
    session::{CPublishFlightState, SPublishFlightState, Session},
    types::{
        IdentifiedQoS, MqttBinary, MqttString, MqttStringPair, PacketIdentifier, QoS, ReasonCode,
        SubscriptionFilter, TopicFilter, TopicName, VarByteInt,
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
pub mod info;
pub mod options;
pub mod raw;

pub use err::Error as MqttError;

/// An MQTT client.
///
/// Configuration via const parameters:
/// - `MAX_SUBSCRIBES`: The maximum amount of in-flight/unacknowledged SUBSCRIBE packets (one per call to [`Self::subscribe`]).
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
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Client<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const MAX_SUBSCRIBES: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
> {
    client_config: ClientConfig,
    shared_config: SharedConfig,
    server_config: ServerConfig,
    session: Session<RECEIVE_MAXIMUM, SEND_MAXIMUM>,

    raw: Raw<'c, N, B>,

    packet_identifier_counter: PacketIdentifier,

    /// sent SUBSCRIBE packets
    pending_suback: Vec<PacketIdentifier, MAX_SUBSCRIBES>,
    /// sent UNSUBSCRIBE packets
    pending_unsuback: Vec<PacketIdentifier, MAX_SUBSCRIBES>,
}

impl<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const MAX_SUBSCRIBES: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
    const MAX_SUBSCRIPTION_IDENTIFIERS: usize,
    const MAX_USER_PROPERTIES: usize,
>
    Client<
        'c,
        N,
        B,
        MAX_SUBSCRIBES,
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
    pub fn new(buffer: &'c mut B) -> Self {
        const {
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

            packet_identifier_counter: PacketIdentifier::ONE,

            pending_suback: Vec::new(),
            pending_unsuback: Vec::new(),
        }
    }

    /// Creates a new, disconnected MQTT client using a buffer provider to store
    /// dynamically sized fields of received packets.
    pub fn with_session(
        session: Session<RECEIVE_MAXIMUM, SEND_MAXIMUM>,
        buffer: &'c mut B,
    ) -> Self {
        let mut s = Self::new(buffer);
        s.session = session;
        s
    }

    /// Returns the amount of publications the client is allowed to make according to the server's
    /// receive maximum. Does not account local space for storing publication state.
    fn remaining_send_quota(&self) -> u16 {
        self.server_config.receive_maximum.into_inner().get() - self.session.in_flight_cpublishes()
    }

    fn is_packet_identifier_used(&self, packet_identifier: PacketIdentifier) -> bool {
        self.session
            .is_used_cpublish_packet_identifier(packet_identifier)
            || self.pending_suback.contains(&packet_identifier)
            || self.pending_unsuback.contains(&packet_identifier)
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
    pub fn session(&self) -> &Session<RECEIVE_MAXIMUM, SEND_MAXIMUM> {
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

    /// Generates a new packet identifier.
    fn packet_identifier(&mut self) -> PacketIdentifier {
        loop {
            let packet_identifier = self.packet_identifier_counter;

            self.packet_identifier_counter = packet_identifier.next();

            if !self.is_packet_identifier_used(packet_identifier) {
                break packet_identifier;
            }
        }
    }

    /// Returns true if the packet identifier exists.
    fn remove_packet_identifier_if_exists<const M: usize>(
        vec: &mut Vec<PacketIdentifier, M>,
        pid: PacketIdentifier,
    ) -> bool {
        if let Some(i) = vec.iter().position(|p| *p == pid) {
            // Safety: The index has just been found in the vector
            unsafe { vec.swap_remove_unchecked(i) };
            true
        } else {
            false
        }
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
    ) -> Result<ConnectInfo<'d, MAX_USER_PROPERTIES>, MqttError<'c, MAX_USER_PROPERTIES>>
    where
        'c: 'd,
    {
        assert!(
            options.user_properties.len() <= MAX_USER_PROPERTIES,
            "Attempted to send CONNECT with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );
        if let Some(ref will) = options.will {
            assert!(
                will.user_properties.len() <= MAX_USER_PROPERTIES,
                "Attempted to send Will with {} > {} (MAX_USER_PROPERTIES) properties",
                will.user_properties.len(),
                MAX_USER_PROPERTIES
            );
        }

        if options.clean_start {
            self.session.clear();
        }

        self.pending_suback.clear();
        self.pending_unsuback.clear();

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
                    "Every MQTT packet is at least 2 bytes long, a smaller maximum packet size makes no sense"
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

            if !session_present && !options.clean_start {
                info!("server does not have the requested session present.");
                self.session.clear();
            }

            let client_identifier = assigned_client_identifier
                .map(Property::into_inner)
                .or(client_identifier)
                .ok_or_else(|| {
                    error!("server did not assign a client identifier when it was required.");
                    self.raw.close_with(Some(ReasonCode::ProtocolError));
                    MqttError::Server
                })?;

            self.shared_config.session_expiry_interval =
                session_expiry_interval.unwrap_or(options.session_expiry_interval);
            self.shared_config.keep_alive =
                server_keep_alive.map_or(options.keep_alive, Property::into_inner);

            if let Some(r) = receive_maximum {
                self.server_config.receive_maximum = r;
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

            info!("connected to server (session present: {})", session_present);

            Ok(ConnectInfo {
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
    /// A subscription identifier should only be set if the server supports
    /// subscription identifiers (Can be checked with [`Self::server_config`]).
    /// The client does not double-check whether this feature is supported and will
    /// always include the subscription identifier argument if present.
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
            "Attempted to send SUBSCRIBE with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        if self.pending_suback.is_full() {
            info!("maximum concurrent subscriptions reached");
            return Err(MqttError::SessionBuffer);
        }

        let subscribe_filter = SubscriptionFilter::new(topic_filter, options);

        let pid = self.packet_identifier();
        let subscribe_filters = [subscribe_filter].into();
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

        // `!self.pending_suback.is_full` guarantees there is space
        self.pending_suback.push(pid).unwrap();

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
            "Attempted to send UNSUBSCRIBE with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        if self.pending_unsuback.is_full() {
            info!("maximum concurrent unsubscriptions reached");
            return Err(MqttError::SessionBuffer);
        }

        let pid = self.packet_identifier();
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

        // `!self.pending_unsuback.is_full` guarantees there is space
        self.pending_unsuback.push(pid).unwrap();

        Ok(pid)
    }

    /// Publish a message. If [`QoS`] is greater than 0, the packet identifier is also kept track of by the client
    ///
    /// # Returns:
    /// - In case of [`QoS`] 0: [`None`]
    /// - In case of [`QoS`] 1 or 2: [`Some`] with the packet identifier of the published packet
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::SendQuotaExceeded`] if the server's control flow limit is reached and sending
    ///   the PUBLISH would exceed the limit causing a protocol error
    /// * [`MqttError::SessionBuffer`] if the buffer for outgoing PUBLISH packet identifiers is full
    /// * [`MqttError::InvalidTopicAlias`] if a topic alias is used and
    ///   * its value is 0
    ///   * its value is greater than the server's maximum topic alias
    /// * [`MqttError::PacketMaximumLengthExceeded`] if the PUBLISH packet is too long to be encoded
    ///   with MQTT's [`VarByteInt`]
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this PUBLISH packet
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
            "Attempted to publish {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        if options.qos > QoS::AtMostOnce {
            if self.remaining_send_quota() == 0 {
                info!("server receive maximum reached");
                return Err(MqttError::SendQuotaExceeded);
            }
            if self.session.cpublish_remaining_capacity() == 0 {
                info!("client maximum concurrent publications reached");
                return Err(MqttError::SessionBuffer);
            }
        }

        let identified_qos = match options.qos {
            QoS::AtMostOnce => IdentifiedQoS::AtMostOnce,
            QoS::AtLeastOnce => IdentifiedQoS::AtLeastOnce(self.packet_identifier()),
            QoS::ExactlyOnce => IdentifiedQoS::ExactlyOnce(self.packet_identifier()),
        };

        if options
            .topic
            .alias()
            .is_some_and(|a| !(1..=self.server_config.topic_alias_maximum).contains(&a.get()))
        {
            return Err(MqttError::InvalidTopicAlias);
        }

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

        // Treat the packet as sent before successfully sending. In case of a network error,
        // we have tracked the packet as in flight and can republish it.
        let pid = match identified_qos {
            IdentifiedQoS::AtMostOnce => None,
            IdentifiedQoS::AtLeastOnce(packet_identifier) => Some({
                // Safety: `cpublish_remaining_capacity()` > 0 confirms that there is space.
                unsafe { self.session.await_puback(packet_identifier) };
                packet_identifier
            }),
            IdentifiedQoS::ExactlyOnce(packet_identifier) => Some({
                // Safety: `cpublish_remaining_capacity()` > 0 confirms that there is space.
                unsafe { self.session.await_pubrec(packet_identifier) };
                packet_identifier
            }),
        };

        match identified_qos.packet_identifier() {
            Some(pid) => debug!("sending PUBLISH packet with packet identifier {}", pid),
            None => debug!("sending PUBLISH packet"),
        }

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(pid)
    }

    /// Resends a PUBLISH packet with DUP flag set.
    ///
    /// This method must be called and must only be called after a reconnection with clean start set to 0,
    /// as resending packets at any other time is a protocol error.
    /// (Compare [Message delivery retry](https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901238), \[MQTT-4.4.0-1\]).
    ///
    /// For a packet to be resent:
    /// - it must have a quality of service > 0
    /// - its packet identifier must have an in flight entry with a quality of service matching the
    ///   quality of service in the options parameter
    /// - in case of quality of service 2, it must not already be awaiting a PUBCOMP packet
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::RepublishQoSNotMatching`] if the [`QoS`] of this republish does not match the
    ///   [`QoS`] that this packet identifier was originally published with    
    /// * [`MqttError::PacketIdentifierAwaitingPubcomp`] if a PUBREC packet with this packet identifier
    ///   has already been received and the server has therefore already received the PUBLISH
    /// * [`MqttError::PacketIdentifierNotInFlight`] if this packet identifier is not tracked in the
    ///   client's session
    /// * [`MqttError::InvalidTopicAlias`] if a topic alias is used and
    ///   * its value is 0
    ///   * its value is greater than the server's maximum topic alias
    /// * [`MqttError::PacketMaximumLengthExceeded`] if the PUBLISH packet is too long to be encoded
    ///   with MQTT's [`VarByteInt`]
    /// * [`MqttError::ServerMaximumPacketSizeExceeded`] if the server's maximum packet size would be
    ///   exceeded by sending this PUBLISH packet
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
            "Attempted to publish {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

        if options.qos == QoS::AtMostOnce {
            panic!("QoS 0 packets cannot be republished");
        }

        let identified_qos = match self.session.cpublish_flight_state(packet_identifier) {
            Some(CPublishFlightState::AwaitingPuback) if options.qos == QoS::AtLeastOnce => {
                IdentifiedQoS::AtLeastOnce(packet_identifier)
            }
            Some(CPublishFlightState::AwaitingPubrec) if options.qos == QoS::ExactlyOnce => {
                IdentifiedQoS::ExactlyOnce(packet_identifier)
            }

            Some(CPublishFlightState::AwaitingPuback) => {
                warn!(
                    "packet identifier {} was originally published with QoS 1",
                    packet_identifier
                );
                return Err(MqttError::RepublishQoSNotMatching);
            }
            Some(CPublishFlightState::AwaitingPubrec) => {
                warn!(
                    "packet identifier {} was originally published with QoS 2",
                    packet_identifier
                );
                return Err(MqttError::RepublishQoSNotMatching);
            }
            Some(CPublishFlightState::AwaitingPubcomp) => {
                warn!(
                    "packet identifier {} is already awaiting PUBCOMP",
                    packet_identifier
                );
                return Err(MqttError::PacketIdentifierAwaitingPubcomp);
            }
            None => {
                warn!("packet identifier {} not in flight", packet_identifier);
                return Err(MqttError::PacketIdentifierNotInFlight);
            }
        };

        if options
            .topic
            .alias()
            .is_some_and(|a| !(1..=self.server_config.topic_alias_maximum).contains(&a.get()))
        {
            return Err(MqttError::InvalidTopicAlias);
        }

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

        // We only republish a message if its quality of service and flight state is correct.
        // In this case, we don't have to change its in flight tracking state as it already
        // is in the desired state.

        debug!(
            "resending PUBLISH packet with packet identifier {}",
            packet_identifier
        );

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Resends all pending PUBREL packets.
    ///
    /// This method must be called and must only be called after a reconnection
    /// with clean start set to 0, as resending packets at any other time is a protocol error.
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
        for packet_identifier in self
            .session
            .pending_client_publishes
            .iter()
            .filter(|s| s.state == CPublishFlightState::AwaitingPubcomp)
            .map(|p| p.packet_identifier)
        {
            let pubrel = PubrelPacket::<0>::new(packet_identifier, ReasonCode::Success);

            // Don't check whether length exceeds servers maximum packet size because we don't
            // add properties to PUBREL packets -> length is always minimal at 6 bytes.
            // The server really shouldn't reject this.
            self.raw.send(&pubrel).await?;
        }

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
    #[inline]
    pub async fn abort(&mut self) {
        match self.raw.abort().await {
            Ok(()) => info!("connection aborted"),
            Err(e) => warn!("connection abort failed: {:?}", e),
        }
    }

    /// Disconnects gracefully from the server by sending a DISCONNECT packet.
    ///
    /// # Preconditions:
    /// - The client did not return a non-recoverable Error before
    ///
    /// # Errors
    ///
    /// * [`MqttError::RecoveryRequired`] if an unrecoverable error occured previously
    /// * [`MqttError::Network`] if the underlying [`Transport`] returned an error
    /// * [`MqttError::IllegalDisconnectSessionExpiryInterval`] if the session expiry interval in the
    ///   CONNECT packet was zero and the session expiry interval in the [`DisconnectOptions`] is [`Some`]
    ///   and not [`SessionExpiryInterval::EndOnDisconnect`].
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
            "Attempted to send DISCONNECT with {} > {} (MAX_USER_PROPERTIES) properties",
            options.user_properties.len(),
            MAX_USER_PROPERTIES
        );

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

        let reason_code = if options.publish_will {
            ReasonCode::DisconnectWithWillMessage
        } else {
            ReasonCode::Success
        };

        let mut packet = DisconnectPacket::<0>::new(
            reason_code,
            options
                .user_properties
                .iter()
                .map(MqttStringPair::as_borrowed)
                .map(Into::into)
                .collect(),
        );
        if let Some(s) = options.session_expiry_interval {
            packet.add_session_expiry_interval(s);
        }

        debug!("sending DISCONNECT packet");

        // Don't check whether length exceeds servers maximum packet size because we don't
        // add a reason string to the DISCONNECT packet -> length is always in the 4..=9 range in bytes.
        // The server really shouldn't reject this.
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
    ///   * the server exceeded the client's receive maximum with a new [`QoS`] 2 PUBLISH
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

                if Self::remove_packet_identifier_if_exists(&mut self.pending_suback, pid) {
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

                if Self::remove_packet_identifier_if_exists(&mut self.pending_unsuback, pid) {
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

                match publish.identified_qos {
                    IdentifiedQoS::AtMostOnce => {
                        debug!("received QoS 0 publication");

                        Event::Publish(publish)
                    }
                    IdentifiedQoS::AtLeastOnce(pid) => {
                        debug!("received QoS 1 publication with packet identifier {}", pid);

                        // We could disconnect here using ReasonCode::ReceiveMaximumExceeded, but incoming QoS 1 publications
                        // don't require resources outside of this scope which means we can just accept these packets.

                        let puback = PubackPacket::<0>::new(pid, ReasonCode::Success);

                        debug!("sending PUBACK packet");

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to PUBACK packets -> length is always minimal at 6 bytes.
                        // The server really shouldn't reject this.
                        self.raw.send(&puback).await?;
                        self.raw.flush().await?;

                        Event::Publish(publish)
                    }
                    IdentifiedQoS::ExactlyOnce(pid) => {
                        debug!("received QoS 2 publication with packet identifier {}", pid);

                        let event = match self.session.spublish_flight_state(pid) {
                            Some(SPublishFlightState::AwaitingPubrel) => Event::Duplicate,
                            None if self.session.spublish_remaining_capacity() > 0 => {
                                // Safety: `spublish_remaining_capacity()` > 0 confirms that there is space.
                                unsafe { self.session.await_pubrel(pid) };
                                Event::Publish(publish)
                            }
                            None => {
                                error!("server exceeded receive maximum");
                                self.raw
                                    .close_with(Some(ReasonCode::ReceiveMaximumExceeded));
                                return Err(MqttError::Server);
                            }
                        };

                        let pubrec = PubrecPacket::<0>::new(pid, ReasonCode::Success);

                        debug!("sending PUBREC packet");

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to PUBREC packets -> length is always minimal at 6 bytes.
                        // The server really shouldn't reject this.
                        self.raw.send(&pubrec).await?;
                        self.raw.flush().await?;

                        event
                    }
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

                let pid = puback.packet_identifier;
                let reason_code = puback.reason_code;

                match self.session.remove_cpublish(pid) {
                    Some(CPublishFlightState::AwaitingPuback) if reason_code.is_success() => {
                        debug!("publication with packet identifier {} complete", pid);

                        Event::PublishAcknowledged(Puback::from(puback))
                    }
                    Some(CPublishFlightState::AwaitingPuback) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej::from(puback))
                    }
                    Some(
                        s @ CPublishFlightState::AwaitingPubrec
                        | s @ CPublishFlightState::AwaitingPubcomp,
                    ) => {
                        warn!("packet identifier {} in PUBACK is actually {:?}", pid, s);

                        // Readd this packet identifier to the session so that it can be republished
                        // after reconnecting.

                        // Safety: Session::remove_cpublish returning Some and therefore successfully
                        // removing a cpublish frees space to add a new in flight entry.
                        unsafe { self.session.r#await(pid, s) };

                        error!("received mismatched PUBACK");
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                    None => {
                        debug!("packet identifier {} in PUBACK not in use", pid);
                        Event::Ignored
                    }
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

                let pid = pubrec.packet_identifier;
                let reason_code = pubrec.reason_code;

                match self.session.remove_cpublish(pid) {
                    Some(CPublishFlightState::AwaitingPubrec) if reason_code.is_success() => {
                        // Safety: Session::remove_cpublish returning Some and therefore successfully
                        // removing a cpublish frees space to add a new in flight entry.
                        unsafe { self.session.await_pubcomp(pid) };

                        let pubrel = PubrelPacket::<0>::new(pid, ReasonCode::Success);

                        debug!("sending PUBREL packet");

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to PUBREL packets -> length is always minimal at 6 bytes.
                        // The server really shouldn't reject this.
                        self.raw.send(&pubrel).await?;
                        self.raw.flush().await?;

                        Event::PublishReceived(Puback::from(pubrec))
                    }
                    Some(CPublishFlightState::AwaitingPubrec) => {
                        // After receiving an erroneous PUBREC, we have to treat any subsequent PUBLISH packet
                        // with the same packet identifier as a new message. This is achieved by already having
                        // removed the packet identifier's in flight entry.

                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej::from(pubrec))
                    }
                    Some(
                        s @ CPublishFlightState::AwaitingPuback
                        | s @ CPublishFlightState::AwaitingPubcomp,
                    ) => {
                        warn!("packet identifier {} in PUBREC is actually {:?}", pid, s);

                        // Readd this packet identifier to the session so that it can be republished
                        // after reconnecting.

                        // Safety: Session::remove_cpublish returning Some and therefore successfully
                        // removing a cpublish frees space to add a new in flight entry.
                        unsafe { self.session.r#await(pid, s) };

                        error!("received mismatched PUBREC");
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                    None => {
                        debug!("packet identifier {} in PUBREC not in use", pid);

                        let pubrel =
                            PubrelPacket::<0>::new(pid, ReasonCode::PacketIdentifierNotFound);

                        debug!("sending PUBREL packet");

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to PUBREL packets -> length is always minimal at 6 bytes.
                        // The server really shouldn't reject this.
                        self.raw.send(&pubrel).await?;
                        self.raw.flush().await?;

                        Event::Ignored
                    }
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

                let pid = pubrel.packet_identifier;
                let reason_code = pubrel.reason_code;

                match self.session.remove_spublish(pid) {
                    Some(SPublishFlightState::AwaitingPubrel) if reason_code.is_success() => {
                        let pubcomp = PubcompPacket::<0>::new(pid, ReasonCode::Success);

                        debug!("sending PUBCOMP packet");

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to PUBCOMP packets -> length is always minimal at 6 bytes.
                        // The server really shouldn't reject this.
                        self.raw.send(&pubcomp).await?;
                        self.raw.flush().await?;

                        Event::PublishReleased(Puback::from(pubrel))
                    }
                    Some(SPublishFlightState::AwaitingPubrel) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej::from(pubrel))
                    }
                    None => {
                        debug!("packet identifier {} in PUBREL not in use", pid);

                        let pubcomp =
                            PubcompPacket::<0>::new(pid, ReasonCode::PacketIdentifierNotFound);

                        debug!("sending PUBCOMP packet");

                        // Don't check whether length exceeds servers maximum packet size because we don't
                        // add properties to PUBCOMP packets -> length is always minimal at 6 bytes.
                        // The server really shouldn't reject this.
                        self.raw.send(&pubcomp).await?;
                        self.raw.flush().await?;

                        Event::Ignored
                    }
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

                let pid = pubcomp.packet_identifier;
                let reason_code = pubcomp.reason_code;

                match self.session.remove_cpublish(pid) {
                    Some(CPublishFlightState::AwaitingPubcomp) if reason_code.is_success() => {
                        debug!("publication with packet identifier {} complete", pid);

                        Event::PublishComplete(Puback::from(pubcomp))
                    }
                    Some(CPublishFlightState::AwaitingPubcomp) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej::from(pubcomp))
                    }
                    Some(
                        s @ CPublishFlightState::AwaitingPuback
                        | s @ CPublishFlightState::AwaitingPubrec,
                    ) => {
                        warn!("packet identifier {} in PUBCOMP is actually {:?}", pid, s);

                        // Readd this packet identifier to the session so that it can be republished
                        // after reconnecting.

                        // Safety: Session::remove_cpublish returning Some and therefore successfully
                        // removing a cpublish frees space to add a new in flight entry.
                        unsafe { self.session.r#await(pid, s) };

                        error!("received mismatched PUBCOMP");
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                    None => {
                        debug!("packet identifier {} in PUBCOMP not in use", pid);
                        Event::Ignored
                    }
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
