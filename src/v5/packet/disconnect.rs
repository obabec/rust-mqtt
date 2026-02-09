#[cfg(test)]
use crate::types::MqttString;
use crate::{
    buffer::BufferProvider,
    config::SessionExpiryInterval,
    eio::{Read, Write},
    fmt::{error, trace},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{ReasonCode, VarByteInt},
    v5::property::{AtMostOnceProperty, PropertyType, ReasonString, ServerReference},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DisconnectPacket<'p> {
    pub reason_code: ReasonCode,

    /// Never sent by server
    pub session_expiry_interval: Option<SessionExpiryInterval>,
    pub reason_string: Option<ReasonString<'p>>,
    pub server_reference: Option<ServerReference<'p>>,
}

impl<'p> Packet for DisconnectPacket<'p> {
    const PACKET_TYPE: PacketType = PacketType::Disconnect;
}
impl<'p> RxPacket<'p> for DisconnectPacket<'p> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding");

        if header.flags() != 0 {
            error!("flags are not 0");
            return Err(RxError::MalformedPacket);
        }

        let r = &mut reader;

        let disconnect_reason_code = if header.remaining_len.size() == 0 {
            trace!("received minimal packet");
            ReasonCode::Success
        } else {
            trace!("reading disconnect reason code");
            ReasonCode::read(r).await?
        };

        if !matches!(
            disconnect_reason_code,
            ReasonCode::Success
                | ReasonCode::DisconnectWithWillMessage
                | ReasonCode::UnspecifiedError
                | ReasonCode::MalformedPacket
                | ReasonCode::ProtocolError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::NotAuthorized
                | ReasonCode::ServerBusy
                | ReasonCode::ServerShuttingDown
                | ReasonCode::BadAuthenticationMethod
                | ReasonCode::KeepAliveTimeout
                | ReasonCode::SessionTakenOver
                | ReasonCode::TopicFilterInvalid
                | ReasonCode::TopicNameInvalid
                | ReasonCode::ReceiveMaximumExceeded
                | ReasonCode::TopicAliasInvalid
                | ReasonCode::PacketTooLarge
                | ReasonCode::MessageRateTooHigh
                | ReasonCode::QuotaExceeded
                | ReasonCode::AdministrativeAction
                | ReasonCode::PayloadFormatInvalid
                | ReasonCode::RetainNotSupported
                | ReasonCode::QoSNotSupported
                | ReasonCode::UseAnotherServer
                | ReasonCode::ServerMoved
                | ReasonCode::SharedSubscriptionsNotSupported
                | ReasonCode::ConnectionRateExceeded
                | ReasonCode::MaximumConnectTime
                | ReasonCode::SubscriptionIdentifiersNotSupported
                | ReasonCode::WildcardSubscriptionsNotSupported
        ) {
            error!("invalid reason code: {:?}", disconnect_reason_code);
            return Err(RxError::ProtocolError);
        }

        let mut packet = Self {
            reason_code: disconnect_reason_code,
            session_expiry_interval: None,
            reason_string: None,
            server_reference: None,
        };

        let properties_length = if header.remaining_len.size() < 2 {
            trace!("received packet with implicit properties length");
            0
        } else {
            trace!("reading properties length");
            VarByteInt::read(r).await?.size()
        };

        trace!("properties length = {}", properties_length);

        if r.remaining_len() != properties_length {
            error!("properties length is not equal to remaining packet length");
            return Err(RxError::MalformedPacket);
        }

        while r.remaining_len() > 0 {
            trace!(
                "reading property type with remaining len = {}",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;

            trace!(
                "reading property body of {:?} with remaining len = {}",
                property_type,
                r.remaining_len()
            );
            #[rustfmt::skip]
            match property_type {
                PropertyType::ReasonString => packet.reason_string.try_set(r).await?,
                PropertyType::ServerReference => packet.server_reference.try_set(r).await?,
                PropertyType::UserProperty => {
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                },
                // Protocol error according to [MQTT-3.14.2-2]
                PropertyType::SessionExpiryInterval => return Err(RxError::ProtocolError),
                // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                p => {
                    error!("packet contains unexpected property {:?}", p);
                    return Err(RxError::MalformedPacket)
                },
            };
        }

        Ok(packet)
    }
}
impl<'p> TxPacket for DisconnectPacket<'p> {
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x00, self.remaining_len())
            .write(write)
            .await?;

        self.reason_code.write(write).await?;

        let properties_length = self.properties_length();
        properties_length.write(write).await?;

        self.session_expiry_interval.write(write).await?;
        self.reason_string.write(write).await?;
        self.server_reference.write(write).await?;

        Ok(())
    }

    fn remaining_len(&self) -> VarByteInt {
        let variable_header_length = wlen!(ReasonCode);

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let total_length = variable_header_length + total_properties_length;

        // Invariant: Max length: 131086 < VarByteInt::MAX_ENCODABLE
        // variable header (reason_code): 1
        // properties length: 4
        // properties: 131081
        VarByteInt::new_unchecked(total_length as u32)
    }
}

impl<'p> DisconnectPacket<'p> {
    pub const fn new(reason_code: ReasonCode) -> Self {
        Self {
            reason_code,
            session_expiry_interval: None,
            reason_string: None,
            server_reference: None,
        }
    }

    pub fn add_session_expiry_interval(&mut self, session_expiry_interval: SessionExpiryInterval) {
        self.session_expiry_interval = Some(session_expiry_interval);
    }

    #[cfg(test)]
    pub fn add_reason_string(&mut self, reason_string: MqttString<'p>) {
        self.reason_string = Some(reason_string.into());
    }

    fn properties_length(&self) -> VarByteInt {
        let len = self.session_expiry_interval.written_len()
            + self.reason_string.written_len()
            + self.server_reference.written_len();

        // Invariant: Max length = 131081 < VarByteInt::MAX_ENCODABLE
        // session expiry interval: 5
        // reason string: 65538
        // server reference: 65538
        VarByteInt::new_unchecked(len as u32)
    }
}

#[cfg(test)]
mod unit {
    use crate::{
        config::SessionExpiryInterval,
        test::{rx::decode, tx::encode},
        types::{MqttString, ReasonCode},
        v5::{
            packet::DisconnectPacket,
            property::{ReasonString, ServerReference},
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_simple() {
        let packet = DisconnectPacket::new(ReasonCode::MaximumConnectTime);

        #[rustfmt::skip]
        encode!(packet, [
            0xE0, //
            0x02, // remaining length
            0xA0, //
            0x00, //
        ]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_properties() {
        let mut packet = DisconnectPacket::new(ReasonCode::MaximumConnectTime);
        packet.add_session_expiry_interval(SessionExpiryInterval::Seconds(23089475));
        packet.add_reason_string(MqttString::try_from("Accroitre Momentum").unwrap());

        #[rustfmt::skip]
        encode!(packet, [
                0xE0, //
                0x1C, // remaining length
                0xA0, // reason code
                0x1A, // property length
                // Session Expiry Interval
                0x11, 0x01, 0x60, 0x51, 0x43, // Reason String
                0x1F, 0x00, 0x12, b'A', b'c', b'c', b'r', b'o', b'i', b't', b'r', b'e', b' ', b'M',
                b'o', b'm', b'e', b'n', b't', b'u', b'm',
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_zero_session_expiry_interval() {
        let mut packet = DisconnectPacket::new(ReasonCode::MaximumConnectTime);
        packet.add_session_expiry_interval(SessionExpiryInterval::EndOnDisconnect);

        #[rustfmt::skip]
        encode!(packet, [
                0xE0, //
                0x07, // remaining length
                0xA0, // reason code
                0x05, // property length

                0x11, 0x00, 0x00, 0x00, 0x00, // Session Expiry Interval
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_simple() {
        let packet = decode!(DisconnectPacket, 2, [0xE0, 0x02, 0x94, 0x00]);

        assert_eq!(packet.reason_code, ReasonCode::TopicAliasInvalid);
        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.server_reference.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_abbreviated() {
        let packet = decode!(DisconnectPacket, 1, [0xE0, 0x01, 0x8E]);

        assert_eq!(packet.reason_code, ReasonCode::SessionTakenOver);
        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.server_reference.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_minimal() {
        let packet = decode!(DisconnectPacket, 0, [0xE0, 0x00]);

        assert_eq!(packet.reason_code, ReasonCode::Success);
        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.server_reference.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(DisconnectPacket, 50, [
            0xE0,
            0x32,
            0x04, // Reason code
            0x30, // Property length

            // Reason String
            0x1F, 0x00, 0x08, b'd', b'e', b'a', b'd', b'b', b'e', b'e', b'f',

            // User Property
            0x26, 0x00, 0x03, b'l', b'o', b'l',
                  0x00, 0x03, b'h', b'e', b'y',

            // User Property
            0x26, 0x00, 0x03, b'l', b'o', b'l',
                  0x00, 0x04, b'b', b'a', b'n', b'g',

            // Server Reference
            0x1C, 0x00, 0x0B, b'g', b'o', b'-', b'h', b'e', b'r', b'e', b'.', b'c', b'o', b'm',
        ]);

        assert_eq!(packet.reason_code, ReasonCode::DisconnectWithWillMessage);
        assert!(packet.session_expiry_interval.is_none());
        assert_eq!(
            packet.reason_string,
            Some(ReasonString(MqttString::try_from("deadbeef").unwrap()))
        );
        assert_eq!(
            packet.server_reference,
            Some(ServerReference(
                MqttString::try_from("go-here.com").unwrap()
            ))
        );
    }
}
