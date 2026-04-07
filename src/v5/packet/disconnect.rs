use heapless::Vec;

#[cfg(test)]
use crate::types::MqttString;
use crate::{
    buffer::BufferProvider,
    config::SessionExpiryInterval,
    eio::{Read, Write},
    fmt::{trace, verbose},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{ReasonCode, VarByteInt},
    v5::property::{AtMostOnceProperty, PropertyType, ReasonString, ServerReference, UserProperty},
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DisconnectPacket<'p, const MAX_USER_PROPERTIES: usize> {
    pub reason_code: ReasonCode,

    /// Never sent by server
    pub session_expiry_interval: Option<SessionExpiryInterval>,
    pub reason_string: Option<ReasonString<'p>>,
    pub user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
    pub server_reference: Option<ServerReference<'p>>,
}

impl<const MAX_USER_PROPERTIES: usize> Packet for DisconnectPacket<'_, MAX_USER_PROPERTIES> {
    const PACKET_TYPE: PacketType = PacketType::Disconnect;
}
impl<'p, const MAX_USER_PROPERTIES: usize> RxPacket<'p>
    for DisconnectPacket<'p, MAX_USER_PROPERTIES>
{
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding DISCONNECT packet");

        if header.flags() != 0 {
            trace!("invalid DISCONNECT fixed header flags: {}", header.flags());
            return Err(RxError::MalformedPacket);
        }

        let r = &mut reader;

        let disconnect_reason_code = if header.remaining_len.size() == 0 {
            verbose!("received minimal DISCONNECT packet");
            ReasonCode::Success
        } else {
            verbose!("reading reason code field");
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
            trace!(
                "invalid DISCONNECT reason code: {:?}",
                disconnect_reason_code
            );
            return Err(RxError::ProtocolError);
        }

        let properties_length = if header.remaining_len.size() < 2 {
            verbose!("DISCONNECT packet has implicit property length = 0");
            0
        } else {
            verbose!("reading property length field");
            VarByteInt::read(r).await?.size()
        };

        verbose!("property length: {} bytes", properties_length);

        if r.remaining_len() != properties_length {
            trace!("invalid DISCONNECT property length for remaining packet length");
            return Err(RxError::MalformedPacket);
        }

        let mut reason_string = None;
        let mut user_properties = Vec::new();
        let mut server_reference = None;

        while r.remaining_len() > 0 {
            verbose!(
                "reading property identifier (remaining length: {} bytes)",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;

            verbose!(
                "reading {:?} property body (remaining length: {} bytes)",
                property_type,
                r.remaining_len()
            );
            #[rustfmt::skip]
            match property_type {
                // Protocol error according to [MQTT-3.14.2-2]
                PropertyType::SessionExpiryInterval => return Err(RxError::ProtocolError),
                PropertyType::ReasonString => reason_string.try_set(r).await?,
                PropertyType::UserProperty if !user_properties.is_full() => {
                    let user_property = UserProperty::read(r).await?;

                    // Safety: `!Vec::is_full` guarantees there is space
                    unsafe { user_properties.push_unchecked(user_property) };
                },
                PropertyType::UserProperty => {
                    UserProperty::skip(r).await?;
                },
                PropertyType::ServerReference => server_reference.try_set(r).await?,
                // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                p => {
                    trace!("invalid DISCONNECT property: {:?}", p);
                    return Err(RxError::MalformedPacket)
                },
            };
        }

        let packet = Self {
            reason_code: disconnect_reason_code,
            session_expiry_interval: None,
            reason_string,
            user_properties,
            server_reference,
        };

        Ok(packet)
    }
}
impl<const MAX_USER_PROPERTIES: usize> TxPacket for DisconnectPacket<'_, MAX_USER_PROPERTIES> {
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x00, self.remaining_len())
            .write(write)
            .await?;

        self.reason_code.write(write).await?;

        let properties_length = self.properties_length();
        properties_length.write(write).await?;

        self.session_expiry_interval.write(write).await?;
        self.reason_string.write(write).await?;

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        self.server_reference.write(write).await?;

        Ok(())
    }

    fn remaining_len(&self) -> VarByteInt {
        let variable_header_length = wlen!(ReasonCode);

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let total_length = variable_header_length + total_properties_length;

        // max length = MAX_USER_PROPERTIES * 131077 + 131086
        // Invariant: MAX_USER_PROPERTIES <= 2046 => max length <= VarByteInt::MAX_ENCODABLE
        // variable header (reason_code): 1
        // property length: 4
        // properties: MAX_USER_PROPERTIES * 131077 + 131081
        VarByteInt::new_unchecked(total_length as u32)
    }
}

impl<'p, const MAX_USER_PROPERTIES: usize> DisconnectPacket<'p, MAX_USER_PROPERTIES> {
    pub const fn new(
        reason_code: ReasonCode,
        user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
    ) -> Self {
        Self {
            reason_code,
            session_expiry_interval: None,
            reason_string: None,
            user_properties,
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
            + self
                .user_properties
                .iter()
                .map(Writable::written_len)
                .sum::<usize>()
            + self.server_reference.written_len();

        // max length = MAX_USER_PROPERTIES * 131077 + 131081
        // Invariant: MAX_USER_PROPERTIES <= 2046 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // session expiry interval: 5
        // reason string: 65538
        // user properties: MAX_USER_PROPERTIES * 131077
        // server reference: 65538
        VarByteInt::new_unchecked(len as u32)
    }
}

#[cfg(test)]
mod unit {
    use heapless::Vec;

    use crate::{
        config::SessionExpiryInterval,
        test::{rx::decode, tx::encode},
        types::{MqttString, MqttStringPair, ReasonCode},
        v5::{
            packet::DisconnectPacket,
            property::{ReasonString, ServerReference, UserProperty},
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_simple() {
        let packet = DisconnectPacket::<16>::new(ReasonCode::MaximumConnectTime, Vec::new());

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
        let mut packet = DisconnectPacket::<16>::new(
            ReasonCode::MaximumConnectTime,
            [
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("status").unwrap(),
                    MqttString::from_str("dead").unwrap(),
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("cause").unwrap(),
                    MqttString::from_str("tripped on the wifi").unwrap(),
                )),
            ]
            .into(),
        );
        packet.add_session_expiry_interval(SessionExpiryInterval::Seconds(23089475));
        packet.add_reason_string(MqttString::try_from("Accroitre Momentum").unwrap());

        #[rustfmt::skip]
        encode!(packet, [
                0xE0, //
                0x48, // remaining length
                0xA0, // reason code
                0x46, // property length
                
                // Session Expiry Interval
                0x11, 0x01, 0x60, 0x51, 0x43, 
                
                // Reason String
                0x1F, 0x00, 0x12, b'A', b'c', b'c', b'r', b'o', b'i', b't', b'r', b'e', b' ', b'M',
                b'o', b'm', b'e', b'n', b't', b'u', b'm',

                0x26,       // User property
                0x00, 0x06, b's', b't', b'a', b't', b'u', b's', 
                0x00, 0x04, b'd', b'e', b'a', b'd',

                0x26,       // User property
                0x00, 0x05, b'c', b'a', b'u', b's', b'e', 
                0x00, 0x13, b't', b'r', b'i', b'p', b'p', b'e', b'd', b' ', b'o', b'n', b' ', b't', b'h', b'e', b' ', b'w', b'i', b'f', b'i', 
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_zero_session_expiry_interval() {
        let mut packet = DisconnectPacket::<16>::new(ReasonCode::MaximumConnectTime, Vec::new());
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
        let packet = decode!(DisconnectPacket<16>, 2, [0xE0, 0x02, 0x94, 0x00]);

        assert_eq!(packet.reason_code, ReasonCode::TopicAliasInvalid);
        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.user_properties.is_empty());
        assert!(packet.server_reference.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_abbreviated() {
        let packet = decode!(DisconnectPacket<16>, 1, [0xE0, 0x01, 0x8E]);

        assert_eq!(packet.reason_code, ReasonCode::SessionTakenOver);
        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.user_properties.is_empty());
        assert!(packet.server_reference.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_minimal() {
        let packet = decode!(DisconnectPacket<16>, 0, [0xE0, 0x00]);

        assert_eq!(packet.reason_code, ReasonCode::Success);
        assert!(packet.session_expiry_interval.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.user_properties.is_empty());
        assert!(packet.server_reference.is_none());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(DisconnectPacket<16>, 50, [
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
            packet.user_properties.as_slice(),
            &[
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("lol").unwrap(),
                    MqttString::from_str("hey").unwrap()
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("lol").unwrap(),
                    MqttString::from_str("bang").unwrap()
                ))
            ]
        );
        assert_eq!(
            packet.server_reference,
            Some(ServerReference(
                MqttString::try_from("go-here.com").unwrap()
            ))
        );
    }
}
