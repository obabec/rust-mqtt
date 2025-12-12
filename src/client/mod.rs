//! Implements full client functionality with session and configuration handling and Quality of Service flows.

use core::hint::unreachable_unchecked;

use crate::{
    buffer::BufferProvider,
    bytes::Bytes,
    client::{
        event::{Event, Puback, Publish, Pubrej, Suback},
        info::ConnectInfo,
        options::{ConnectOptions, DisconnectOptions, PublicationOptions, SubscriptionOptions},
        raw::Raw,
    },
    config::{ClientConfig, ServerConfig, SessionExpiryInterval, SharedConfig},
    fmt::{debug, error, panic, warn},
    header::{FixedHeader, PacketType},
    io::net::Transport,
    packet::Packet,
    session::{CPublishFlightState, SPublishFlightState, Session},
    types::{MqttString, QoS, ReasonCode, Topic, TopicFilter},
    v5::{
        packet::{
            ConnackPacket, ConnectPacket, DisconnectPacket, PingreqPacket, PingrespPacket,
            PubackPacket, PubcompPacket, PublishPacket, PubrecPacket, PubrelPacket, SubackPacket,
            SubscribePacket, UnsubackPacket, UnsubscribePacket,
        },
        property::Property,
    },
};
use heapless::Vec;

mod err;

pub mod event;
pub mod info;
pub mod options;
pub mod raw;

pub use err::Error as MqttError;

/// An MQTT client.
#[derive(Debug)]
pub struct Client<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const MAX_SUBSCRIBES: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> {
    client_config: ClientConfig,
    shared_config: SharedConfig,
    server_config: ServerConfig,
    session: Session<RECEIVE_MAXIMUM, SEND_MAXIMUM>,

    raw: Raw<'c, N, B>,

    packet_identifier_counter: u16,

    /// sent SUBSCRIBE packets
    pending_suback: Vec<u16, MAX_SUBSCRIBES>,
    /// sent UNSUBSCRIBE packets
    pending_unsuback: Vec<u16, MAX_SUBSCRIBES>,
}

impl<
    'c,
    N: Transport,
    B: BufferProvider<'c>,
    const MAX_SUBSCRIBES: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> Client<'c, N, B, MAX_SUBSCRIBES, RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    /// Creates a new, disconnected MQTT client using a buffer provider to store
    /// dynamically sized fields of received packets.
    /// The session state is initialised as a new session. If you want to start the
    /// client with an existing session, use `Client::with_session()`
    pub fn new(buffer: &'c mut B) -> Self {
        Self {
            client_config: ClientConfig::default(),
            shared_config: SharedConfig::default(),
            server_config: ServerConfig::default(),
            session: Session::default(),

            raw: Raw::new_disconnected(buffer),

            packet_identifier_counter: 1,

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
        self.server_config.receive_maximum.into_inner() - self.session.in_flight_cpublishes()
    }

    fn is_packet_identifier_used(&self, packet_identifier: u16) -> bool {
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

    /// Returns a mutable reference to the supplied `BufferProvider` implementation.
    ///
    /// This can for example be used to reset the underlying buffer if using `BumpBuffer`.
    #[inline]
    pub fn buffer(&mut self) -> &mut B {
        self.raw.buffer()
    }

    /// Generates a new packet identifier.
    fn packet_identifier(&mut self) -> u16 {
        loop {
            let packet_identifier = self.packet_identifier_counter;
            self.packet_identifier_counter = match self.packet_identifier_counter {
                u16::MAX => 1,
                i => i + 1,
            };

            if !self.is_packet_identifier_used(packet_identifier) {
                break packet_identifier;
            }
        }
    }

    /// Returns true if the packet identifier exists.
    fn remove_packet_identifier_if_exists<const M: usize>(vec: &mut Vec<u16, M>, pid: u16) -> bool {
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
    /// - a non-recoverable error has occured and Client::abort() has been called.
    /// - Client::disconnect() has been called.
    ///
    /// The session expiry interval in ConnectOptions overrides the one in the session of the client.
    ///
    /// Configuration that was negotiated with the server is stored in the `client_config`, `server_config`,
    /// `shared_config` and `session` fields which have getters.
    ///
    /// # Returns:
    /// Information not being used currently by the client and therefore stored in its fields.
    pub async fn connect<'d>(
        &mut self,
        net: N,
        options: &ConnectOptions<'_>,
        client_identifier: Option<MqttString<'d>>,
    ) -> Result<ConnectInfo<'d>, MqttError<'c>>
    where
        'c: 'd,
    {
        if options.clean_start {
            self.session.clear();
        }

        self.pending_suback.clear();
        self.pending_unsuback.clear();

        self.raw.set_net(net);

        // Set client session expiry interval because it is relevant to determine
        // which session expiry interval can be sent in DISCONNECT packet.
        self.client_config.session_expiry_interval = options.session_expiry_interval;

        {
            let packet_client_identifier = client_identifier
                .as_ref()
                .map(|s| s.as_borrowed())
                .unwrap_or_default();

            let mut packet = ConnectPacket::new(
                packet_client_identifier,
                options.clean_start,
                options.keep_alive,
                options.session_expiry_interval,
                RECEIVE_MAXIMUM as u16,
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

                packet.add_will(will.as_will(), will_qos, will_retain);
            }

            debug!("sending CONNECT packet");
            self.raw.send(&packet).await?;
            self.raw.flush().await?;
        }

        debug!("awaiting CONNACK packet header");

        let header = self.raw.recv_header().await?;

        match header.packet_type() {
            Ok(ConnackPacket::PACKET_TYPE) => {}
            Ok(_) => {
                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::Server);
            }
            Err(_) => {
                error!("received invalid header {:?}", header);
                self.raw.close_with(Some(ReasonCode::MalformedPacket));
                return Err(MqttError::Server);
            }
        }

        debug!("awaiting CONNACK packet body");

        let ConnackPacket {
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
            wildcard_subscription_available,
            subscription_identifier_available,
            shared_subscription_available,
            server_keep_alive,
            response_information: _,
            server_reference: _,
            authentication_method: _,
            authentication_data: _,
        } = self.raw.recv_body(&header).await?;

        debug!("received CONNACK packet");

        if reason_code.is_success() {
            debug!("CONNACK packet indicates success");

            let client_identifier = match assigned_client_identifier {
                Some(c) => c.into_inner(),
                None => match client_identifier {
                    Some(c) => c,
                    None => {
                        error!("server did not assign a client identifier when it was required.");
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                },
            };

            self.shared_config.session_expiry_interval =
                session_expiry_interval.unwrap_or(options.session_expiry_interval);
            self.shared_config.keep_alive = server_keep_alive
                .map(Property::into_inner)
                .unwrap_or(options.keep_alive);

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

            Ok(ConnectInfo {
                session_present,
                client_identifier,
            })
        } else {
            debug!("CONNACK packet indicates rejection");

            self.raw.close_with(None);

            Err(MqttError::Disconnect {
                reason: reason_code,
                reason_string: reason_string.map(Property::into_inner),
            })
        }
    }

    /// Start a ping handshake by sending a PINGRESP packet.
    pub async fn ping(&mut self) -> Result<(), MqttError<'c>> {
        debug!("sending PINGREQ packet");

        self.raw.send(&PingreqPacket::new()).await?;
        self.raw.flush().await?;

        Ok(())
    }

    /// Subscribes to a single topic with the given options.
    ///
    /// The client keeps track of the packet identifier sent in the SUBSCRIBE packet.
    /// If no `Event::Suback` is received within a custom time,
    /// this method can be used to send the SUBSCRIBE packet again.
    ///
    /// # Returns:
    /// - The packet identifier of the sent SUBSCRIBE packet.
    pub async fn subscribe(
        &mut self,
        topic: Topic<'_>,
        options: SubscriptionOptions,
    ) -> Result<u16, MqttError<'c>> {
        if self.pending_suback.len() == MAX_SUBSCRIBES {
            warn!("maximum concurrent subscriptions reached");
            return Err(MqttError::SessionBuffer);
        }

        let topic_filter = TopicFilter::new(topic, &options);

        let pid = self.packet_identifier();
        let mut topic_filters = Vec::<_, 1>::new();
        let _ = topic_filters.push(topic_filter);
        let packet = SubscribePacket::new(pid, topic_filters);

        debug!("sending SUBSCRIBE packet");

        self.raw.send(&packet).await?;
        self.raw.flush().await?;
        self.pending_suback.push(pid).unwrap();

        Ok(pid)
    }

    /// Unsubscribes from a single topic.
    ///
    /// The client keeps track of the packet identifier sent in the UNSUBSCRIBE packet.
    /// If no `Event::Unsuback` is received within a custom time,
    /// this method can be used to send the UNSUBSCRIBE packet again.
    ///
    /// # Returns:
    /// - The packet identifier of the sent UNSUBSCRIBE packet.
    pub async fn unsubscribe(&mut self, topic: Topic<'_>) -> Result<u16, MqttError<'c>> {
        if self.pending_unsuback.len() == MAX_SUBSCRIBES {
            warn!("maximum concurrent unsubscriptions reached");
            return Err(MqttError::SessionBuffer);
        }

        let pid = self.packet_identifier();
        let mut topics = Vec::<_, 1>::new();
        let _ = topics.push(topic);
        let packet = UnsubscribePacket::new(pid, topics);

        debug!("sending UNSUBSCRIBE packet");

        self.raw.send(&packet).await?;
        self.raw.flush().await?;
        self.pending_unsuback.push(pid).unwrap();

        Ok(pid)
    }

    /// Publish a message. If QoS is greater than 0, the packet identifier is also kept track of by the client
    ///
    /// # Returns:
    /// - In case of QoS 0, a packet identifier of 0 is returned.
    ///   This value is not allowed in MQTT and is an escape value without semantic meaning.
    /// - In case of Qos 1 or 2 the packet identifier of the published packet
    pub async fn publish(
        &mut self,
        options: &PublicationOptions<'_>,
        message: Bytes<'_>,
    ) -> Result<u16, MqttError<'c>> {
        if options.qos > QoS::AtMostOnce {
            if self.remaining_send_quota() == 0 {
                warn!("server receive maximum reached");
                return Err(MqttError::SendQuotaExceeded);
            }
            if self.session.cpublish_remaining_capacity() == 0 {
                warn!("client maximum concurrent publications reached");
                return Err(MqttError::SessionBuffer);
            }
        }

        let mut packet = PublishPacket::new(
            false,
            options.retain,
            options.topic.as_ref().as_borrowed(),
            message,
        )?;

        let packet_identifier = match options.qos {
            QoS::AtMostOnce => {
                debug!("sending PUBLISH packet");
                0
            }
            _ => {
                let pid = self.packet_identifier();
                debug!("sending PUBLISH packet with packet identifier {}", pid);
                packet.set_qos(options.qos, pid);
                pid
            }
        };

        match options.qos {
            QoS::AtMostOnce => {}
            QoS::AtLeastOnce => {
                // Safety: `remaining_send_quota()` > 0 confirms that there is space.
                unsafe { self.session.await_puback(packet_identifier) };
            }
            QoS::ExactlyOnce => {
                // Safety: `remaining_send_quota()` > 0 confirms that there is space.
                unsafe { self.session.await_pubrec(packet_identifier) };
            }
        }

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        Ok(packet_identifier)
    }

    /// Resends a PUBLISH packet with DUP flag set.
    ///
    /// This method should (or MUST in order to be MQTT compliant) be called after reconnecting to an existing session.
    ///
    /// # Preconditions:
    /// - QoS in PublicationOptions > 0
    /// - Packet Identifier is in flight
    /// - The client did not return a non-recoverable Error before
    pub async fn republish(
        &mut self,
        packet_identifier: u16,
        options: &PublicationOptions<'_>,
        message: Bytes<'_>,
    ) -> Result<(), MqttError<'c>> {
        let s = (options.qos > QoS::AtMostOnce)
            .then(|| self.session.remove_cpublish(packet_identifier))
            .flatten();

        match (options.qos, s) {
            (QoS::AtMostOnce, _) => panic!("QoS 0 packets cannot be republished"),
            // Safety: A single entry was just removed so there is space.
            (QoS::AtLeastOnce, Some(_)) => unsafe { self.session.await_puback(packet_identifier) },
            // Safety: A single entry was just removed so there is space.
            (QoS::ExactlyOnce, Some(_)) => unsafe { self.session.await_pubrec(packet_identifier) },
            (_, None) => {
                warn!(
                    "packet identifier {} not in flight or not in correct in-flight state",
                    packet_identifier
                );
                return Err(MqttError::PacketIdentifierNotInFlight);
            }
        }

        let mut packet = PublishPacket::new(
            true,
            options.retain,
            options.topic.as_ref().as_borrowed(),
            message,
        )?;

        packet.set_qos(options.qos, packet_identifier);

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
    /// This method should (or MUST in order to be MQTT compliant) be called after reconnecting to an existing session.
    pub async fn rerelease(&mut self) -> Result<(), MqttError<'c>> {
        for packet_identifier in self
            .session
            .pending_client_publishes
            .iter()
            .filter(|s| s.state == CPublishFlightState::AwaitingPubcomp)
            .map(|p| p.packet_identifier)
        {
            let pubrel = PubrelPacket::new(packet_identifier, ReasonCode::Success);

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
    /// This is not cancel-safe but you can set a timeout if the network connection is replaced with `Client::` down the line or you don't reuse the client.
    #[inline]
    pub async fn abort(&mut self) {
        #[allow(unused_must_use)]
        self.raw.abort().await;
    }

    /// Disconnects gracefully from the server by sending a DISCONNECT packet.
    ///
    /// # Preconditions:
    /// - The client did not return a non-recoverable Error before
    pub async fn disconnect(&mut self, options: &DisconnectOptions) -> Result<(), MqttError<'c>> {
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

        let mut packet = DisconnectPacket::new(reason_code);
        if let Some(s) = options.session_expiry_interval {
            packet.add_session_expiry_interval(s);
        }

        debug!("sending DISCONNECT packet");

        self.raw.send(&packet).await?;
        self.raw.flush().await?;

        // Terminates (closes) the connection by dropping it
        self.raw.close_with(None);

        debug!("disconnected from server");

        Ok(())
    }

    /// Combines `Client::poll_header` and `Client::poll_body`.
    ///
    /// Polls the network for a full packet. Not cancel-safe.
    ///
    /// # Preconditions:
    /// - The last MQTT packet was received completely
    /// - The client did not return a non-recoverable Error before
    ///
    /// # Returns:
    /// - MQTT Events. Their further meaning is documented in `Event`
    pub async fn poll(&mut self) -> Result<Event<'c>, MqttError<'c>> {
        let header = self.poll_header().await?;
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
    /// - The received fixed header with a valid packet type. It can be used to call `poll_body`
    pub async fn poll_header(&mut self) -> Result<FixedHeader, MqttError<'c>> {
        let header = self.raw.recv_header().await?;

        match header.packet_type() {
            Ok(p) => debug!("received header of {:?}", p),
            Err(_) => {
                error!("received invalid header {:?}", header);
                self.raw.close_with(Some(ReasonCode::MalformedPacket));
                return Err(MqttError::Server);
            }
        }

        Ok(header)
    }

    /// Polls the network for the variable header and payload of a packet. Not cancel-safe.
    ///
    /// # Preconditions:
    /// - The FixedHeader argument was received from the network right before.
    /// - The client did not return a non-recoverable Error before
    ///
    /// # Returns:
    /// - MQTT Events for regular communication. Their further meaning is documented in `Event`.
    /// - `MqttError::Disconnect` when receiving a DISCONNECT packet.
    pub async fn poll_body(&mut self, header: FixedHeader) -> Result<Event<'c>, MqttError<'c>> {
        let event = match header.packet_type()? {
            PacketType::Pingresp => {
                debug!("receiving PINGRESP packet");
                self.raw.recv_body::<PingrespPacket>(&header).await?;
                Event::Pingresp
            }
            PacketType::Suback => {
                debug!("receiving SUBACK packet");
                let suback = self.raw.recv_body::<SubackPacket<'_, 1>>(&header).await?;
                let pid = suback.packet_identifier;

                if Self::remove_packet_identifier_if_exists(&mut self.pending_suback, pid) {
                    // We only send SUBSCRIBE packets with exactly 1 topic
                    if suback.reason_codes.len() != 1 {
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                    }
                    let r = suback.reason_codes.first().unwrap();

                    Event::Suback(Suback {
                        packet_identifier: pid,
                        reason_code: *r,
                    })
                } else {
                    warn!("packet identifier {} in SUBACK not in use", pid);
                    Event::Ignored
                }
            }
            PacketType::Unsuback => {
                debug!("receiving UNSUBACK packet");
                let unsuback = self.raw.recv_body::<UnsubackPacket<'_, 1>>(&header).await?;
                let pid = unsuback.packet_identifier;

                if Self::remove_packet_identifier_if_exists(&mut self.pending_unsuback, pid) {
                    // We only send UNSUBSCRIBE packets with exactly 1 topic
                    if unsuback.reason_codes.len() != 1 {
                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                    }

                    let r = unsuback.reason_codes.first().unwrap();

                    Event::Unsuback(Suback {
                        packet_identifier: pid,
                        reason_code: *r,
                    })
                } else {
                    warn!("packet identifier {} in UNSUBACK not in use", pid);
                    Event::Ignored
                }
            }
            PacketType::Publish => {
                debug!("receiving PUBLISH packet");
                let publish = self.raw.recv_body::<PublishPacket>(&header).await?;
                let pid = publish.packet_identifier;

                let event = Event::Publish(Publish {
                    packet_identifier: pid.unwrap_or_default(),
                    dup: publish.dup,
                    retain: publish.retain,
                    qos: publish.qos,
                    topic: publish.topic,
                    message: publish.message,
                });

                match pid {
                    None if publish.qos == QoS::AtMostOnce => {
                        debug!("received QoS 0 publication");

                        event
                    }
                    // Safety: `PublishPacket::receive` always returns a packet identifier if QoS != 0
                    None => unsafe { unreachable_unchecked() },
                    Some(pid) if self.session.is_used_spublish_packet_identifier(pid) => {
                        warn!("packet identifier {} already in use", pid);
                        Event::Ignored
                    }
                    Some(pid) => match publish.qos {
                        // Safety: `PublishPacket::receive` doesn't read packet identifier if QoS > 0
                        QoS::AtMostOnce => unsafe { unreachable_unchecked() },
                        QoS::AtLeastOnce => {
                            debug!("received QoS 1 publication with packet identifier {}", pid);

                            // Could disconnect using ReasonCode::ReceiveMaximumExceeded, but we can handle a QoS 1 publication always.

                            let puback = PubackPacket::new(pid, ReasonCode::Success);

                            debug!("sending PUBACK packet");

                            self.raw.send(&puback).await?;
                            self.raw.flush().await?;

                            event
                        }
                        QoS::ExactlyOnce => {
                            debug!("received QoS 2 publication with packet identifier {}", pid);

                            if self.session.spublish_remaining_capacity() == 0 {
                                error!("server exceeded receive maximum");
                                self.raw
                                    .close_with(Some(ReasonCode::ReceiveMaximumExceeded));
                                return Err(MqttError::Server);
                            }

                            let pubrec = PubrecPacket::new(pid, ReasonCode::Success);

                            // Safety: `spublish_capacity()` > 0 confirms that there is space.
                            unsafe { self.session.await_pubrel(pid) };

                            debug!("sending PUBREC packet");

                            self.raw.send(&pubrec).await?;
                            self.raw.flush().await?;

                            event
                        }
                    },
                }
            }
            PacketType::Puback => {
                debug!("receiving PUBACK packet");
                let puback = self.raw.recv_body::<PubackPacket>(&header).await?;
                let pid = puback.packet_identifier;
                let reason_code = puback.reason_code;

                match self.session.remove_cpublish(pid) {
                    Some(CPublishFlightState::AwaitingPuback) if reason_code.is_success() => {
                        debug!("publication with packet identifier {} complete", pid);

                        Event::PublishAcknowledged(Puback {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    Some(CPublishFlightState::AwaitingPuback) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    Some(s) => {
                        warn!("packet identifier {} in PUBACK is actually {:?}", pid, s);

                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                    None => {
                        warn!("packet identifier {} in PUBACK not in use", pid);
                        Event::Ignored
                    }
                }
            }
            PacketType::Pubrec => {
                debug!("receiving PUBREC packet");
                let pubrec = self.raw.recv_body::<PubrecPacket>(&header).await?;
                let pid = pubrec.packet_identifier;
                let reason_code = pubrec.reason_code;

                match self.session.remove_cpublish(pid) {
                    Some(
                        CPublishFlightState::AwaitingPubrec | CPublishFlightState::AwaitingPubcomp,
                    ) if reason_code.is_success() => {
                        let pubrel = PubrelPacket::new(pid, ReasonCode::Success);

                        debug!("sending PUBREL packet");

                        self.raw.send(&pubrel).await?;
                        self.raw.flush().await?;

                        // Safety: successfully removing a cpublish frees space for a new in flight.
                        unsafe { self.session.await_pubcomp(pid) };

                        Event::PublishReceived(Puback {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    Some(
                        CPublishFlightState::AwaitingPubrec | CPublishFlightState::AwaitingPubcomp,
                    ) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    Some(CPublishFlightState::AwaitingPuback) => {
                        warn!(
                            "packet identifier {} in PUBREC is actually awaiting PUBACK",
                            pid
                        );

                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                    None => {
                        warn!("packet identifier {} in PUBREC not in use", pid);
                        Event::Ignored
                    }
                }
            }
            PacketType::Pubrel => {
                debug!("receiving PUBREL packet");
                let pubrel = self.raw.recv_body::<PubrelPacket>(&header).await?;
                let pid = pubrel.packet_identifier;
                let reason_code = pubrel.reason_code;

                match self.session.remove_spublish(pid) {
                    Some(SPublishFlightState::AwaitingPubrel) if reason_code.is_success() => {
                        let pubcomp = PubcompPacket::new(pid, ReasonCode::Success);

                        debug!("sending PUBCOMP packet");

                        self.raw.send(&pubcomp).await?;
                        self.raw.flush().await?;

                        Event::PublishReleased(Puback {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    Some(SPublishFlightState::AwaitingPubrel) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    None => {
                        warn!("packet identifier {} in PUBREL not in use", pid);
                        Event::Ignored
                    }
                }
            }
            PacketType::Pubcomp => {
                debug!("receiving PUBCOMP packet");
                let pubcomp = self.raw.recv_body::<PubcompPacket>(&header).await?;
                let pid = pubcomp.packet_identifier;
                let reason_code = pubcomp.reason_code;

                match self.session.remove_cpublish(pid) {
                    Some(CPublishFlightState::AwaitingPubcomp) if reason_code.is_success() => {
                        debug!("publication with packet identifier {} complete", pid);

                        Event::PublishComplete(Puback {
                            packet_identifier: pid,
                            reason_code: pubcomp.reason_code,
                        })
                    }
                    Some(CPublishFlightState::AwaitingPubcomp) => {
                        debug!("publication with packet identifier {} aborted", pid);

                        Event::PublishRejected(Pubrej {
                            packet_identifier: pid,
                            reason_code,
                        })
                    }
                    Some(s) => {
                        warn!("packet identifier {} in PUBCOMP is actually {:?}", pid, s);

                        self.raw.close_with(Some(ReasonCode::ProtocolError));
                        return Err(MqttError::Server);
                    }
                    None => {
                        warn!("packet identifier {} in PUBCOMP not in use", pid);
                        Event::Ignored
                    }
                }
            }
            PacketType::Disconnect => {
                debug!("receiving DISCONNECT packet");
                let disconnect = self.raw.recv_body::<DisconnectPacket>(&header).await?;

                return Err(MqttError::Disconnect {
                    reason: disconnect.reason_code,
                    reason_string: disconnect.reason_string.map(Property::into_inner),
                });
            }
            t @ (PacketType::Connect
            | PacketType::Subscribe
            | PacketType::Unsubscribe
            | PacketType::Pingreq) => {
                error!("received packet server is not allowed to send: {:?}", t);

                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::Server);
            }
            PacketType::Connack => {
                error!("received CONNACK packet");

                self.raw.close_with(Some(ReasonCode::ProtocolError));
                return Err(MqttError::Server);
            }
            PacketType::Auth => {
                error!("received AUTH packet");

                self.raw
                    .close_with(Some(ReasonCode::ImplementationSpecificError));
                return Err(MqttError::AuthPacketReceived);
            }
        };

        Ok(event)
    }
}
