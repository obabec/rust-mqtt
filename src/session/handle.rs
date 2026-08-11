use crate::{
    client::options::AckMode,
    fmt::{assert, assert_ne, panic, trace},
    session::{Error, Event, LocalPublishState, PeerPublishState, Response, Session},
    types::{PacketIdentifier, QoS, ReasonCode},
};

pub struct FreeHandle<
    'a,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> {
    pub session: &'a mut Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,
    pub packet_identifier: PacketIdentifier,
}

impl<const SUBSCRIBE_MAXIMUM: usize, const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize>
    FreeHandle<'_, SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    pub fn outbound_sub(self) -> Result<(), Error> {
        self.session
            .subs
            .push(self.packet_identifier)
            .inspect(|_| {
                trace!(
                    "initiating subscription {{ pid=#{} }}",
                    self.packet_identifier
                )
            })
            .map_err(|_| {
                trace!(
                    "preventing subscription due to a lack of buffer space {{ pid=#{} }}",
                    self.packet_identifier
                );

                Error::NoCapacity
            })
    }
    pub fn outbound_unsub(self) -> Result<(), Error> {
        self.session
            .unsubs
            .push(self.packet_identifier)
            .inspect(|_| {
                trace!(
                    "initiating unsubscription {{ pid=#{} }}",
                    self.packet_identifier
                )
            })
            .map_err(|_| {
                trace!(
                    "preventing unsubscription due to a lack of buffer space {{ pid=#{} }}",
                    self.packet_identifier
                );

                Error::NoCapacity
            })
    }
    pub fn outbound_publish(self, qos: QoS, ack_mode: AckMode) -> Result<(), Error> {
        assert_ne!(qos, QoS::AtMostOnce, "QoS 0 is not to be tracked");
        assert!(
            !(qos == QoS::AtLeastOnce && ack_mode.is_manual()),
            "outbound QoS 1 does not have acknowledgements"
        );

        if self.session.available_outbound_publish_capacity() {
            let initial_state = match qos {
                QoS::AtMostOnce => unreachable!(),
                QoS::AtLeastOnce if ack_mode.is_manual() => unreachable!(),
                QoS::AtLeastOnce => LocalPublishState::AwaitAck,
                QoS::ExactlyOnce => LocalPublishState::AwaitRec(ack_mode),
            };

            trace!(
                "initiating publication {{ pid=#{}, qos={:?} }}",
                self.packet_identifier, qos
            );

            self.session
                .schedule_outbound(self.packet_identifier, initial_state);

            Ok(())
        } else {
            trace!(
                "preventing publication due to a lack of buffer space {{ pid=#{}, qos={:?} }}",
                self.packet_identifier, qos
            );

            Err(Error::NoCapacity)
        }
    }
}

pub struct SubHandle<
    'a,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> {
    pub session: &'a mut Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,
    pub i: usize,
}

impl<const SUBSCRIBE_MAXIMUM: usize, const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize>
    SubHandle<'_, SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    pub(crate) fn remove(self) {
        trace!("#{}: AwaitSuback -> Untracked", self.packet_identifier());

        self.session.subs.swap_remove(self.i);
    }
    fn packet_identifier(&self) -> PacketIdentifier {
        *self.session.subs.get(self.i).unwrap()
    }
}

pub struct UnsubHandle<
    'a,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> {
    pub session: &'a mut Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,
    pub i: usize,
}

impl<const SUBSCRIBE_MAXIMUM: usize, const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize>
    UnsubHandle<'_, SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    pub(crate) fn remove(self) {
        trace!("#{}: AwaitUnsuback -> Untracked", self.packet_identifier());

        self.session.unsubs.swap_remove(self.i);
    }
    fn packet_identifier(&self) -> PacketIdentifier {
        *self.session.unsubs.get(self.i).unwrap()
    }
}

pub struct InboundHandle<
    'a,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> {
    pub session: &'a mut Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,
    pub i: usize,
    pub state: PeerPublishState,
}

impl<const SUBSCRIBE_MAXIMUM: usize, const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize>
    InboundHandle<'_, SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    fn set(&mut self, state: PeerPublishState) {
        trace!(
            "#{}: {:?} -> {:?}",
            self.packet_identifier(),
            self.state,
            state,
        );

        self.state = state;
        self.session.inbound_publishes.get_mut(self.i).unwrap().1 = self.state;
    }
    fn remove(self) {
        trace!(
            "#{}: {:?} -> Untracked",
            self.packet_identifier(),
            self.state,
        );

        self.session.inbound_publishes.swap_remove(self.i);
    }
    fn nop(&self) {
        trace!(
            "#{}: {:?} -> {:?}",
            self.packet_identifier(),
            self.state,
            self.state,
        );
    }
    fn packet_identifier(&self) -> PacketIdentifier {
        self.session.inbound_publishes.get(self.i).unwrap().0
    }

    pub(crate) fn inbound_republish(mut self, qos: QoS) -> (Response, Event) {
        match qos {
            QoS::AtMostOnce => panic!("QoS 0 has no packet identifier, so this call is incorrect"),
            QoS::AtLeastOnce => match self.state {
                PeerPublishState::AwaitPublishExactlyOnce(_)
                | PeerPublishState::DueRec
                | PeerPublishState::AwaitRel(_)
                | PeerPublishState::AwaitReRel
                | PeerPublishState::DueComp => {
                    trace!(
                        "disconnecting on PUBLISH with invalid QoS ({:?}) for tracked publication {{ pid=#{}, qos={:?} }}",
                        qos,
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.nop();

                    (
                        Response::Disconnect(ReasonCode::ProtocolError),
                        Event::ServerError,
                    )
                }

                // The user has not yet sent the manual PUBACK so we leave it up to them.
                // We could consider emitting a duplicate event here because we haven't sent
                // the PUBACK yet undermining the condition of the following normative statement:
                // | After it has sent a PUBACK packet the receiver MUST treat any incoming PUBLISH
                // | packet that contains the same Packet Identifier as being a new Application Message,
                // | irrespective of the setting of its DUP flag [MQTT-4.3.2-5].
                //
                // However, we can still let this message through, which we do.
                PeerPublishState::DueAck => {
                    trace!(
                        "deferring PUBACK for unexpected PUBLISH retransmission {{ pid=#{} }}",
                        self.packet_identifier()
                    );

                    self.nop();

                    (Response::None, Event::Publish)
                }
                PeerPublishState::AwaitPublishAtLeastOnce => {
                    trace!(
                        "deferring PUBACK for PUBLISH retransmission after reconnection {{ pid=#{} }}",
                        self.packet_identifier()
                    );

                    self.set(PeerPublishState::DueAck);

                    (Response::None, Event::Publish)
                }
            },
            QoS::ExactlyOnce => match self.state {
                PeerPublishState::AwaitPublishAtLeastOnce | PeerPublishState::DueAck => {
                    trace!(
                        "disconnecting on PUBLISH with invalid QoS ({:?}) for tracked publication {{ pid=#{}, qos={:?} }}",
                        qos,
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.nop();

                    (
                        Response::Disconnect(ReasonCode::ProtocolError),
                        Event::ServerError,
                    )
                }

                // This state is reached after a reconnection with both possibilities of
                // - the peer resending a PUBLISH
                // - the peer (re-)sending a PUBREL
                //
                // We (the user) has not yet sent a PUBREC in this connection so it is their responsibility.
                PeerPublishState::AwaitPublishExactlyOnce(AckMode::Automatic) => {
                    trace!(
                        "responding with PUBREC to PUBLISH retransmission after reconnection {{ pid=#{}, reason_code={:?} }}",
                        self.packet_identifier(),
                        ReasonCode::Success
                    );

                    self.set(PeerPublishState::AwaitRel(AckMode::Automatic));

                    (
                        Response::Receive(ReasonCode::Success),
                        Event::Duplicate(AckMode::Automatic),
                    )
                }
                PeerPublishState::AwaitPublishExactlyOnce(AckMode::Manual) => {
                    trace!(
                        "deferring PUBREC for PUBLISH retransmission after reconnection {{ pid=#{} }}",
                        self.packet_identifier(),
                    );

                    self.set(PeerPublishState::DueRec);

                    (Response::None, Event::Duplicate(AckMode::Manual))
                }

                // The user has not yet sent the manual PUBREC so we leave it up to them.
                // The PUBLISH has not driven the state forward, so an ignored event would
                // be fitting, but duplicate matches better here.
                PeerPublishState::DueRec => {
                    trace!(
                        "deferring PUBREC for unexpected PUBLISH retransmission {{ pid=#{} }}",
                        self.packet_identifier(),
                    );

                    self.nop();

                    (Response::None, Event::Duplicate(AckMode::Manual))
                }

                // The user has already sent a PUBREC packet, so we send this PUBREC
                // automatically.
                PeerPublishState::AwaitRel(mode) => {
                    trace!(
                        "responding with PUBREC to unexpected PUBLISH retransmission {{ pid=#{}, reason_code={:?} }}",
                        self.packet_identifier(),
                        ReasonCode::Success,
                    );

                    self.nop();

                    (
                        Response::Receive(ReasonCode::Success),
                        Event::Duplicate(mode),
                    )
                }

                // The peer has already sent a PUBREL packet so it must not resend the PUBLISH
                // packet. We have not yet sent a PUBCOMP packet so this PUBLISH also can't be
                // a new application message that reuses the same packet identifier. This is a
                // clear protocol error.
                PeerPublishState::AwaitReRel | PeerPublishState::DueComp => {
                    trace!(
                        "disconnecting on unexpected PUBLISH retransmission because PUBREL was already received {{ pid=#{} }}",
                        self.packet_identifier(),
                    );

                    self.nop();

                    (
                        Response::Disconnect(ReasonCode::ProtocolError),
                        Event::ServerError,
                    )
                }
            },
        }
    }

    /// The PUBACK's [`ReasonCode`] may be successful or erroneous, this doesn't matter
    /// for the state machine as this packet identifier is removed from the session
    /// state in either case.
    pub(crate) fn outbound_puback(self) -> Result<(), Error> {
        match self.state {
            PeerPublishState::AwaitPublishAtLeastOnce => {
                trace!(
                    "preventing PUBACK for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::HandshakeStateMismatched)
            }
            PeerPublishState::AwaitPublishExactlyOnce(_)
            | PeerPublishState::DueRec
            | PeerPublishState::AwaitRel(_)
            | PeerPublishState::AwaitReRel
            | PeerPublishState::DueComp => {
                trace!(
                    "preventing PUBACK for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::QoSMismatched)
            }
            PeerPublishState::DueAck => {
                trace!(
                    "completing publication with PUBACK {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                Ok(())
            }
        }
    }

    pub(crate) fn outbound_pubrec(mut self, reason_code: ReasonCode) -> Result<(), Error> {
        match self.state {
            PeerPublishState::AwaitPublishAtLeastOnce | PeerPublishState::DueAck => {
                trace!(
                    "preventing PUBREC for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::QoSMismatched)
            }
            PeerPublishState::AwaitPublishExactlyOnce(_)
            | PeerPublishState::AwaitRel(_)
            | PeerPublishState::AwaitReRel
            | PeerPublishState::DueComp => {
                trace!(
                    "preventing PUBREC for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::HandshakeStateMismatched)
            }
            PeerPublishState::DueRec if reason_code.is_erroneous() => {
                trace!(
                    "rejecting publication with PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.remove();

                Ok(())
            }
            PeerPublishState::DueRec => {
                trace!(
                    "advancing publication with PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(PeerPublishState::AwaitRel(AckMode::Manual));

                Ok(())
            }
        }
    }

    pub(crate) fn inbound_pubrel(mut self, reason_code: ReasonCode) -> (Response, Event) {
        match self.state {
            // QoS mismatch, the spec doesn't state what to do here. We could
            // - disconnect due to a protocol error (ReasonCode::PacketIdentifierInUse is not allowed for DISCONNECT packets)
            // - send a PUBCOMP, but the only allowed ReasonCode::PacketIdentifierNotFound is not fitting
            PeerPublishState::AwaitPublishAtLeastOnce | PeerPublishState::DueAck => {
                trace!(
                    "disconnecting on PUBREL for tracked publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                (
                    Response::Disconnect(ReasonCode::ProtocolError),
                    Event::ServerError,
                )
            }

            PeerPublishState::AwaitPublishExactlyOnce(_) if reason_code.is_erroneous() => {
                trace!(
                    "completing publication aborted by PUBREL {{ pid=#{}, reason_code={:?} }} with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                (Response::Complete(ReasonCode::Success), Event::Aborted)
            }
            PeerPublishState::AwaitPublishExactlyOnce(AckMode::Automatic) => {
                trace!(
                    "completing publication after PUBREL {{ pid=#{}, reason_code={:?} }} with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                (
                    Response::Complete(ReasonCode::Success),
                    Event::Released(AckMode::Automatic),
                )
            }
            PeerPublishState::AwaitPublishExactlyOnce(AckMode::Manual) => {
                trace!(
                    "deferring PUBCOMP for PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(PeerPublishState::DueComp);

                (Response::None, Event::Released(AckMode::Manual))
            }

            // This state means we have not sent a PUBREC in this connection but received a
            // PUBLISH in this connection. Therefore the peer should not have sent a PUBREL.
            // We have two options:
            // - Accept the release and move the session state forward despite having
            //   skipped the PUBREC. After all, we already delivered the message.
            // - Disconnect due to a protocol error. This risks that the session entry on
            //   our end becomes stale (especially if the reason code of this PUBREL is
            //   negative) because the peer has removed their entry and won't resend the
            //   PUBLISH or PUBREL packet
            PeerPublishState::DueRec if reason_code.is_erroneous() => {
                trace!(
                    "completing publication aborted by PUBREL {{ pid=#{}, reason_code={:?} }} with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                (Response::Complete(ReasonCode::Success), Event::Aborted)
            }
            PeerPublishState::DueRec => {
                trace!(
                    "deferring PUBCOMP for PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(PeerPublishState::DueComp);

                (Response::None, Event::Released(AckMode::Manual))
            }

            PeerPublishState::AwaitRel(_) if reason_code.is_erroneous() => {
                trace!(
                    "completing publication aborted by PUBREL {{ pid=#{}, reason_code={:?} }} with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                (Response::Complete(ReasonCode::Success), Event::Aborted)
            }
            PeerPublishState::AwaitRel(AckMode::Automatic) => {
                trace!(
                    "completing publication after PUBREL {{ pid=#{}, reason_code={:?} }} with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                (
                    Response::Complete(ReasonCode::Success),
                    Event::Released(AckMode::Automatic),
                )
            }
            PeerPublishState::AwaitRel(AckMode::Manual) => {
                trace!(
                    "deferring PUBCOMP for PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(PeerPublishState::DueComp);

                (Response::None, Event::Released(AckMode::Manual))
            }

            PeerPublishState::AwaitReRel => {
                trace!(
                    "deferring PUBCOMP for PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(PeerPublishState::DueComp);

                (Response::None, Event::Released(AckMode::Manual))
            }

            PeerPublishState::DueComp if reason_code.is_erroneous() => {
                trace!(
                    "completing publication aborted by PUBREL {{ pid=#{}, reason_code={:?} }} with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                (Response::Complete(ReasonCode::Success), Event::Aborted)
            }
            PeerPublishState::DueComp => {
                trace!(
                    "deferring PUBCOMP for PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.nop();

                (Response::None, Event::Ignored)
            }
        }
    }

    /// The PUBCOMP's [`ReasonCode`] is assumed to be successful.
    pub(crate) fn outbound_pubcomp(self) -> Result<(), Error> {
        match self.state {
            PeerPublishState::AwaitPublishAtLeastOnce | PeerPublishState::DueAck => {
                trace!(
                    "preventing PUBCOMP for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::QoSMismatched)
            }
            PeerPublishState::AwaitPublishExactlyOnce(_)
            | PeerPublishState::DueRec
            | PeerPublishState::AwaitRel(_)
            | PeerPublishState::AwaitReRel => {
                trace!(
                    "preventing PUBCOMP for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::HandshakeStateMismatched)
            }
            PeerPublishState::DueComp => {
                trace!(
                    "completing publication with PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.remove();

                Ok(())
            }
        }
    }
}

pub struct OutboundHandle<
    'a,
    const SUBSCRIBE_MAXIMUM: usize,
    const RECEIVE_MAXIMUM: usize,
    const SEND_MAXIMUM: usize,
> {
    pub session: &'a mut Session<SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>,
    pub i: usize,
    pub state: LocalPublishState,
}

impl<const SUBSCRIBE_MAXIMUM: usize, const RECEIVE_MAXIMUM: usize, const SEND_MAXIMUM: usize>
    OutboundHandle<'_, SUBSCRIBE_MAXIMUM, RECEIVE_MAXIMUM, SEND_MAXIMUM>
{
    fn set(&mut self, state: LocalPublishState) {
        trace!(
            "#{}: {:?} -> {:?}",
            self.packet_identifier(),
            self.state,
            state,
        );

        self.state = state;
        self.session.outbound_publishes.get_mut(self.i).unwrap().1 = self.state;
    }
    fn remove(self) {
        trace!(
            "#{}: {:?} -> Untracked",
            self.packet_identifier(),
            self.state,
        );

        self.session.outbound_publishes.swap_remove(self.i);
    }
    fn nop(&self) {
        trace!(
            "#{}: {:?} -> {:?}",
            self.packet_identifier(),
            self.state,
            self.state,
        );
    }
    pub(crate) fn packet_identifier(&self) -> PacketIdentifier {
        self.session.outbound_publishes.get(self.i).unwrap().0
    }
    pub(crate) fn next(self) -> Option<Self> {
        let i = self.i + 1;

        self.session
            .outbound_publishes
            .get(i)
            .map(|(p, s)| (*p, *s))
            .map(|(_, state)| Self {
                session: self.session,
                i,
                state,
            })
    }

    pub(crate) fn outbound_republish(&mut self, qos: QoS) -> Result<(), Error> {
        match qos {
            QoS::AtMostOnce => {
                panic!("QoS 0 has no packet identifier, so this call is incorrect");
            }
            QoS::AtLeastOnce => match self.state {
                LocalPublishState::DuePublishAtLeastOnce => {
                    trace!(
                        "retransmitting PUBLISH for publication {{ pid=#{}, qos={:?} }}",
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.set(LocalPublishState::AwaitAck);

                    Ok(())
                }
                LocalPublishState::AwaitAck => {
                    trace!(
                        "preventing PUBLISH retransmission for publication {{ pid=#{}, qos={:?} }}",
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.nop();

                    Err(Error::HandshakeStateMismatched)
                }
                LocalPublishState::DuePublishExactlyOnce(_)
                | LocalPublishState::AwaitRec(_)
                | LocalPublishState::DueRel(_)
                | LocalPublishState::AwaitComp(_) => {
                    trace!(
                        "preventing PUBLISH retransmission with invalid QoS ({:?}) for publication {{ pid=#{}, qos={:?} }}",
                        qos,
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.nop();

                    Err(Error::QoSMismatched)
                }
            },
            QoS::ExactlyOnce => match self.state {
                LocalPublishState::DuePublishAtLeastOnce | LocalPublishState::AwaitAck => {
                    trace!(
                        "preventing PUBLISH retransmission with invalid QoS ({:?}) for publication {{ pid=#{}, qos={:?} }}",
                        qos,
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.nop();

                    Err(Error::QoSMismatched)
                }
                LocalPublishState::AwaitRec(_)
                | LocalPublishState::DueRel(_)
                | LocalPublishState::AwaitComp(_) => {
                    trace!(
                        "preventing PUBLISH retransmission for publication {{ pid=#{}, qos={:?} }}",
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.nop();

                    Err(Error::HandshakeStateMismatched)
                }
                LocalPublishState::DuePublishExactlyOnce(mode) => {
                    trace!(
                        "retransmitting PUBLISH for publication {{ pid=#{}, qos={:?} }}",
                        self.packet_identifier(),
                        QoS::from(self.state),
                    );

                    self.set(LocalPublishState::AwaitRec(mode));

                    Ok(())
                }
            },
        }
    }

    pub(crate) fn inbound_puback(self, reason_code: ReasonCode) -> (Response, Event) {
        match self.state {
            // According to the spec, we MUST retransmit our PUBLISH packet on reconnect,
            // however, for QoS 2 we also accept a PUBREC in the counterpart of this state.
            //
            // The peer should not have sent a PUBACK on reconnect, but our priority is
            // not remote spec enforcement but reliable delivery. The receival of this
            // PUBACK proves that the peer took ownership of the message and delivered it.
            LocalPublishState::DuePublishAtLeastOnce | LocalPublishState::AwaitAck => {
                let e = if reason_code.is_success() {
                    trace!(
                        "completing publication after PUBACK {{ pid=#{}, reason_code={:?} }}",
                        self.packet_identifier(),
                        reason_code,
                    );

                    self.remove();

                    Event::Acknowledged
                } else {
                    trace!(
                        "completing rejected publication after PUBACK {{ pid=#{}, reason_code={:?} }}",
                        self.packet_identifier(),
                        reason_code,
                    );

                    self.remove();

                    Event::Rejected
                };

                (Response::None, e)
            }

            // Mismatched QoS
            LocalPublishState::DuePublishExactlyOnce(_)
            | LocalPublishState::AwaitRec(_)
            | LocalPublishState::DueRel(_)
            | LocalPublishState::AwaitComp(_) => {
                trace!(
                    "disconnecting on PUBACK for tracked publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                (
                    Response::Disconnect(ReasonCode::ProtocolError),
                    Event::ServerError,
                )
            }
        }
    }

    pub(crate) fn inbound_pubrec(mut self, reason_code: ReasonCode) -> (Response, Event) {
        match self.state {
            // Mismatched QoS
            LocalPublishState::DuePublishAtLeastOnce | LocalPublishState::AwaitAck => {
                trace!(
                    "disconnecting on PUBREC for tracked publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                (
                    Response::Disconnect(ReasonCode::ProtocolError),
                    Event::ServerError,
                )
            }

            // Ideally, this state doesn't exist when a PUBREC is received because
            // - on reconnection, all PUBLISH packets should be resent immediately
            // and
            // - the peer must not send a PUBREC packet "out of thin air" after a
            //   reconnection.
            //
            // So we act to stay as conform to the spec as possible. Relevant
            // normative statements:
            // | The sender MUST send a PUBREL packet when it receives a PUBREC packet from the receiver with a Reason Code value less than 0x80 [MQTT-4.3.3-4].
            // | The sender MUST NOT re-send the PUBLISH once it has sent the corresponding PUBREL packet [MQTT-4.3.3-6].
            // | On reconnection, the sender MUST resend any unacknowledged PUBLISH packets [MQTT-4.4.0-1].
            //
            // In our case, after having sent the mandatory PUBREL packet, we treat
            // the PUBLISH packet as acknowledged, which means we don't need to
            // (and must not) resend the PUBLISH packet, which we wouldn't be
            // allowed to anyway because we have sent the PUBREL already.
            LocalPublishState::DuePublishExactlyOnce(_) if reason_code.is_erroneous() => {
                trace!(
                    "aborting rejected publication after PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.remove();

                (Response::None, Event::Rejected)
            }
            LocalPublishState::DuePublishExactlyOnce(AckMode::Automatic) => {
                trace!(
                    "responding to unexpected PUBREC {{ pid=#{}, reason_code={:?} }} with PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.set(LocalPublishState::AwaitComp(AckMode::Automatic));

                (
                    Response::Release(ReasonCode::Success),
                    Event::Received(AckMode::Automatic),
                )
            }
            LocalPublishState::DuePublishExactlyOnce(AckMode::Manual) => {
                trace!(
                    "deferring PUBREL for unexpected PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(LocalPublishState::DueRel(AckMode::Manual));

                (Response::None, Event::Received(AckMode::Manual))
            }

            // Ideally, this state doesn't exist when a PUBREC is received because
            // - the peer must not send a PUBREC packet "out of thin air" after a
            //   reconnection and must not retransmit it during a connection
            // and either of
            // - on reconnection, all PUBREL packets should be resent immediately
            // - in manual ack mode, the PUBREL packet should be sent immediately
            //   after receiving the PUBREC
            LocalPublishState::DueRel(_) if reason_code.is_erroneous() => {
                trace!(
                    "completing rejected publication after unexpected PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.remove();

                (Response::None, Event::Rejected)
            }
            LocalPublishState::DueRel(AckMode::Automatic) => {
                trace!(
                    "responding to unexpected PUBREC {{ pid=#{}, reason_code={:?} }} with PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.set(LocalPublishState::AwaitComp(AckMode::Automatic));

                (Response::Release(ReasonCode::Success), Event::Ignored)
            }
            LocalPublishState::DueRel(AckMode::Manual) => {
                trace!(
                    "deferring PUBREL for unexpected PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.nop();

                // The user has not yet sent the manual PUBREL so we leave it up to them.
                // We do not emit the `Received` event because the user has already seen
                // that event and this PUBREC has not driven the state forward.
                (Response::None, Event::Ignored)
            }

            LocalPublishState::AwaitComp(_) => {
                trace!(
                    "responding to unexpected PUBREC {{ pid=#{}, reason_code={:?} }} with PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.nop();

                // If the ack mode is manual, the user has already sent the PUBREL so we
                // do it automatically now.
                (Response::Release(ReasonCode::Success), Event::Ignored)
            }

            LocalPublishState::AwaitRec(_) if reason_code.is_erroneous() => {
                trace!(
                    "completing rejected publication after PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.remove();

                (Response::None, Event::Rejected)
            }
            LocalPublishState::AwaitRec(AckMode::Automatic) => {
                trace!(
                    "responding to PUBREC {{ pid=#{}, reason_code={:?} }} with PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.set(LocalPublishState::AwaitComp(AckMode::Automatic));

                (
                    Response::Release(ReasonCode::Success),
                    Event::Received(AckMode::Automatic),
                )
            }
            LocalPublishState::AwaitRec(AckMode::Manual) => {
                trace!(
                    "deferring PUBREL for PUBREC {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    reason_code,
                );

                self.set(LocalPublishState::DueRel(AckMode::Manual));

                (Response::None, Event::Received(AckMode::Manual))
            }
        }
    }

    pub(crate) fn outbound_pubrel(&mut self) -> Result<(), Error> {
        match self.state {
            LocalPublishState::DuePublishAtLeastOnce | LocalPublishState::AwaitAck => {
                trace!(
                    "preventing PUBREL for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::QoSMismatched)
            }
            LocalPublishState::DuePublishExactlyOnce(_)
            | LocalPublishState::AwaitRec(_)
            | LocalPublishState::AwaitComp(_) => {
                trace!(
                    "preventing PUBREL for publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                Err(Error::HandshakeStateMismatched)
            }
            LocalPublishState::DueRel(mode) => {
                trace!(
                    "advancing publication with PUBREL {{ pid=#{}, reason_code={:?} }}",
                    self.packet_identifier(),
                    ReasonCode::Success,
                );

                self.set(LocalPublishState::AwaitComp(mode));

                Ok(())
            }
        }
    }

    pub(crate) fn inbound_pubcomp(self, reason_code: ReasonCode) -> (Response, Event) {
        match self.state {
            // Mismatched QoS
            LocalPublishState::DuePublishAtLeastOnce | LocalPublishState::AwaitAck => {
                trace!(
                    "disconnecting on PUBCOMP for tracked publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                (
                    Response::Disconnect(ReasonCode::ProtocolError),
                    Event::ServerError,
                )
            }

            // We must treat the PUBLISH packet as unacknowledged until we have received a PUBREC:
            // | MUST treat the PUBLISH packet as “unacknowledged” until it has received the corresponding PUBREC packet from the receiver [MQTT-4.3.3-3].
            //
            // We have not yet received a PUBREC packet, if we had, we would be in due PUBREL
            // or await PUBCOMP state. Therefore this PUBCOMP can't complete this handshake
            // right now.
            LocalPublishState::DuePublishExactlyOnce(_) | LocalPublishState::AwaitRec(_) => {
                trace!(
                    "disconnecting on unexpected PUBCOMP for tracked publication {{ pid=#{}, qos={:?} }}",
                    self.packet_identifier(),
                    QoS::from(self.state),
                );

                self.nop();

                (
                    Response::Disconnect(ReasonCode::ProtocolError),
                    Event::ServerError,
                )
            }

            LocalPublishState::DueRel(_) | LocalPublishState::AwaitComp(_) => {
                if reason_code.is_success() {
                    trace!(
                        "completing publication after PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                        self.packet_identifier(),
                        reason_code,
                    );

                    self.remove();

                    (Response::None, Event::Completed)
                } else {
                    trace!(
                        "completing rejected publication after PUBCOMP {{ pid=#{}, reason_code={:?} }}",
                        self.packet_identifier(),
                        reason_code,
                    );

                    self.remove();

                    (Response::None, Event::Rejected)
                }
            }
        }
    }
}
