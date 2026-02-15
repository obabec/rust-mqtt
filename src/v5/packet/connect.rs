use crate::{
    config::{KeepAlive, MaximumPacketSize, ReceiveMaximum, SessionExpiryInterval},
    eio::Write,
    header::{FixedHeader, PacketType},
    io::write::{Writable, wlen},
    packet::{Packet, TxError, TxPacket},
    types::{MqttBinary, MqttString, QoS, VarByteInt, Will},
    v5::property::{
        AuthenticationData, AuthenticationMethod, RequestProblemInformation,
        RequestResponseInformation, TopicAliasMaximum,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConnectPacket<'p> {
    // CONNECT connect flags (will flag is implicit due to `will` being an Option<T>)
    will_retain: bool,
    will_qos: QoS,
    clean_start: bool,

    // CONNECT keep alive
    keep_alive: KeepAlive,

    // CONNECT properties
    session_expiry_interval: SessionExpiryInterval,
    receive_maximum: ReceiveMaximum,
    maximum_packet_size: MaximumPacketSize,
    topic_alias_maximum: Option<TopicAliasMaximum>,
    request_response_information: Option<RequestResponseInformation>,
    request_problem_information: Option<RequestProblemInformation>,
    authentication_method: Option<AuthenticationMethod<'p>>,
    authentication_data: Option<AuthenticationData<'p>>,

    // CONNECT payload
    /// Must always be in Connect Payload. Can be length 0
    client_identifier: MqttString<'p>,

    /// Has to be present if will_flag is set
    will: Option<Will<'p>>,

    /// Has to be present if user_name flag is set
    user_name: Option<MqttString<'p>>,
    /// Has to be present if password flag is set
    password: Option<MqttBinary<'p>>,
}
impl<'p> Packet for ConnectPacket<'p> {
    const PACKET_TYPE: PacketType = PacketType::Connect;
}
impl<'p> TxPacket for ConnectPacket<'p> {
    fn remaining_len(&self) -> VarByteInt {
        let variable_header_length = wlen!([u8; 7]) + wlen!(u8) + wlen!(u16);

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let body_length = self.client_identifier.written_len()
            + self.will.written_len()
            + self.user_name.written_len()
            + self.password.written_len();

        let total_length = variable_header_length + total_properties_length + body_length;

        // Invariant: Max length = 393253 < VarByteInt::MAX_ENCODABLE
        // variable header: 8
        // properties length: 4
        // properties: 131093
        // will: 2 * 65537 (will topic & will payload)
        // username: 65537
        // password: 65537
        VarByteInt::new_unchecked(total_length as u32)
    }

    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x00, self.remaining_len())
            .write(write)
            .await?;

        let protocol_name = [0, 4, b'M', b'Q', b'T', b'T', 5];
        let connect_flags = (u8::from(self.user_name.is_some()) << 7)
            | (u8::from(self.password.is_some()) << 6)
            | (u8::from(self.will_retain) << 5)
            | self.will_qos.into_bits(3)
            | (u8::from(self.will.is_some()) << 2)
            | (u8::from(self.clean_start) << 1);

        protocol_name.write(write).await?;
        connect_flags.write(write).await?;
        self.keep_alive.as_u16().write(write).await?;

        let properties_length = self.properties_length();
        properties_length.write(write).await?;

        // If the session expiry interval is 0, it can be omitted on the network.
        // This is only the case for the CONNECT packet.
        if self.session_expiry_interval != SessionExpiryInterval::EndOnDisconnect {
            self.session_expiry_interval.write(write).await?;
        }
        self.receive_maximum.write(write).await?;
        self.maximum_packet_size.write(write).await?;
        self.topic_alias_maximum.write(write).await?;
        self.request_response_information.write(write).await?;
        self.request_problem_information.write(write).await?;
        self.authentication_method.write(write).await?;
        self.authentication_data.write(write).await?;

        self.client_identifier.write(write).await?;
        self.will.write(write).await?;
        self.user_name.write(write).await?;
        self.password.write(write).await?;

        Ok(())
    }
}

impl<'p> ConnectPacket<'p> {
    pub fn new(
        client_identifier: MqttString<'p>,
        clean_start: bool,
        keep_alive: KeepAlive,
        maximum_packet_size: MaximumPacketSize,
        session_expiry_interval: SessionExpiryInterval,
        receive_maximum: u16,
        request_response_information: bool,
    ) -> Self {
        Self {
            will_retain: false,
            will_qos: QoS::AtMostOnce,
            clean_start,
            keep_alive,
            session_expiry_interval,
            receive_maximum: ReceiveMaximum(receive_maximum),
            maximum_packet_size,
            topic_alias_maximum: None,
            request_response_information: request_response_information
                .then_some(true)
                .map(Into::into),
            request_problem_information: None,
            authentication_method: None,
            authentication_data: None,
            client_identifier,
            will: None,
            user_name: None,
            password: None,
        }
    }

    pub fn properties_length(&self) -> VarByteInt {
        let session_expiry_interval_len =
            if self.session_expiry_interval == SessionExpiryInterval::EndOnDisconnect {
                0
            } else {
                self.session_expiry_interval.written_len()
            };

        let len = session_expiry_interval_len
            + self.receive_maximum.written_len()
            + self.maximum_packet_size.written_len()
            + self.topic_alias_maximum.written_len()
            + self.request_response_information.written_len()
            + self.request_problem_information.written_len()
            + self.authentication_method.written_len()
            + self.authentication_data.written_len();

        // Invariant: Max length = 131093 < VarByteInt::MAX_ENCODABLE
        // session expiry interval: 5
        // maximum packet size: 5
        // topic alias maximum: 3
        // request response information: 2
        // request problem information: 2
        // authentication method: 65538
        // authentication data: 65538
        VarByteInt::new_unchecked(len as u32)
    }

    pub fn add_user_name(&mut self, user_name: MqttString<'p>) {
        self.user_name = Some(user_name);
    }
    pub fn add_password(&mut self, password: MqttBinary<'p>) {
        self.password = Some(password);
    }

    pub fn add_will(&mut self, will: Will<'p>, will_qos: QoS, will_retain: bool) {
        self.will_retain = will_retain;
        self.will_qos = will_qos;
        self.will = Some(will);
    }
}

#[cfg(test)]
mod unit {
    use crate::{
        config::{KeepAlive, MaximumPacketSize, SessionExpiryInterval},
        test::tx::encode,
        types::{MqttBinary, MqttString, QoS, TopicName, Will},
        v5::{
            packet::ConnectPacket,
            property::{
                ContentType, MessageExpiryInterval, PayloadFormatIndicator, WillDelayInterval,
            },
        },
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_simple() {
        let packet = ConnectPacket::new(
            MqttString::try_from("a").unwrap(),
            true,
            KeepAlive::Seconds(7439),
            MaximumPacketSize::Unlimited,
            SessionExpiryInterval::EndOnDisconnect,
            u16::MAX,
            false,
        );

        #[rustfmt::skip]
        encode!(packet, [
            0x10,       //
            0x0E,       // remaining length
            0x00,       // ---
            0x04,       //
            b'M',       //
            b'Q',       //
            b'T',       //
            b'T',       // ---
            0x05,       // Protocol version
            0b00000010, // Connect flags
            0x1D,       // Keep alive MSB
            0x0F,       // Keep alive LSB
            0x00,       // Property length
            0x00,       // Client identifier len MSB
            0x01,       // Client identifier len LSB
            b'a',       // Client identifier
        ]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_properties() {
        let packet = ConnectPacket::new(
            MqttString::try_from("a").unwrap(),
            false,
            KeepAlive::Infinite,
            MaximumPacketSize::Limit(2309845),
            SessionExpiryInterval::Seconds(8136391),
            63543,
            true,
        );

        #[rustfmt::skip]
        encode!(packet, [
            0x10,       //
            0x1D,       // remaining length
            0x00,       // ---
            0x04,       //
            b'M',       //
            b'Q',       //
            b'T',       //
            b'T',       // ---
            0x05,       // Protocol version
            0b00000000, // Connect flags
            0x00,       // Keep alive MSB
            0x00,       // Keep alive LSB
            0x0F,       // Property length

            0x11,       // Session expiry interval
            0x00, 0x7C, 0x26, 0xC7,

            0x21,       // Receive maximum
            0xF8, 0x37,

            0x27,       // Maximum packet size
            0x00, 0x23, 0x3E, 0xD5,

            0x19,       // Request response information
            0x01,

            0x00,       // Client identifier len MSB
            0x01,       // Client identifier len LSB
            b'a',       // Client identifier
        ]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_payload() {
        let mut packet = ConnectPacket::new(
            MqttString::try_from("giuqen").unwrap(),
            false,
            KeepAlive::Seconds(6789),
            MaximumPacketSize::Unlimited,
            SessionExpiryInterval::EndOnDisconnect,
            u16::MAX,
            false,
        );

        packet.add_user_name(MqttString::try_from("Franz").unwrap());
        packet.add_password(MqttBinary::try_from("security!".as_bytes()).unwrap());

        #[rustfmt::skip]
        encode!(packet,
            [
                0x10,       //
                0x25,       // remaining length
                0x00,       // ---
                0x04,       //
                b'M',       //
                b'Q',       //
                b'T',       //
                b'T',       // ---
                0x05,       // Protocol version
                0b11000000, // Connect flags
                0x1A,       // Keep alive MSB
                0x85,       // Keep alive LSB
                0x00,       // Property length
                0x00,       // Client identifier len MSB
                0x06,       // Client identifier len LSB
                b'g',       // Client identifier
                b'i',       //
                b'u',       //
                b'q',       //
                b'e',       //
                b'n',       // Client identifier
                0x00, // Username len MSB
                0x05, // Username len LSB
                b'F', //
                b'r', //
                b'a', //
                b'n', //
                b'z', // Username
                0x00, // Password len MSB
                0x09, // Password len LSB
                b's', //
                b'e', //
                b'c', //
                b'u', //
                b'r', //
                b'i', //
                b't', //
                b'y', //
                b'!', // Password
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_will() {
        let mut packet = ConnectPacket::new(
            MqttString::try_from("cba").unwrap(),
            false,
            KeepAlive::Seconds(6789),
            MaximumPacketSize::Unlimited,
            SessionExpiryInterval::Seconds(893475),
            u16::MAX,
            true,
        );

        packet.add_user_name(MqttString::try_from("Franz").unwrap());
        packet.add_password(MqttBinary::try_from("security!".as_bytes()).unwrap());
        packet.add_will(
            Will {
                will_topic: TopicName::new(MqttString::try_from("dead").unwrap()).unwrap(),
                will_delay_interval: Some(WillDelayInterval(234589)),
                payload_format_indicator: Some(PayloadFormatIndicator(true)),
                message_expiry_interval: Some(MessageExpiryInterval(84807612)),
                content_type: Some(ContentType(MqttString::try_from("text/plain").unwrap())),
                response_topic: None,
                correlation_data: None,
                will_payload: MqttBinary::try_from([12, 8, 98].as_slice()).unwrap(),
            },
            QoS::ExactlyOnce,
            true,
        );

        #[rustfmt::skip]
        encode!(packet,
            [
                0x10,       // Packet type
                0x4E,       // Remaining length
                0x00,       // Protocol name len MSB
                0x04,       // Protocol name len LSB
                b'M',       // Protocol name
                b'Q',       //
                b'T',       //
                b'T',       // Protocol name
                0x05,       // Protocol version
                0b11110100, // Connect flags
                0x1A,       // Keep alive MSB
                0x85,       // Keep alive LSB

                0x07,       // Property length

                0x11, 0x00, 0x0D, 0xA2, 0x23, // Session expiry interval

                0x19, 0x01, // Request Response Information

                0x00,       // Client identifier len MSB
                0x03,       // Client identifier len LSB
                b'c',       // Client identifier
                b'b',       //
                b'a',       // Client identifier

                0x19,       // Will properties length

                0x18, 0x00, 0x03, 0x94, 0x5D, // Will delay interval

                0x01, 0x01,       // Payload format indicator

                0x02, 0x05, 0x0E, 0x0F, 0xBC, // Message expiry interval

                0x03, 0x00, 0x0A, // Content type
                b't', b'e', b'x', b't', b'/', b'p', b'l', b'a', b'i', b'n', // Content type

                0x00,       // Will topic len MSB
                0x04,       // Will topic len LSB
                b'd',       // Will topic
                b'e',       //
                b'a',       //
                b'd',       // Will topic

                0x00,       // Will payload len MSB
                0x03,       // Will payload len LSB
                12, 8, 98,  // Will payload

                0x00,       // Username len MSB
                0x05,       // Username len LSB
                b'F',       // Username
                b'r',       //
                b'a',       //
                b'n',       //
                b'z',       // Username
                
                0x00,       // Password len MSB
                0x09,       // Password len LSB
                b's',       // Password
                b'e',       //
                b'c',       //
                b'u',       //
                b'r',       //
                b'i',       //
                b't',       //
                b'y',       //
                b'!',       // Password
            ]
        );
    }
}
