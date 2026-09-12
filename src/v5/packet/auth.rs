use heapless::Vec;

use crate::{
    buffer::BufferProvider,
    eio::{Read, Write},
    fmt::{const_assert, trace, verbose},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{ReasonCode, VarByteInt},
    v5::property::{
        AtMostOnceProperty, AuthenticationData, AuthenticationMethod, PropertyType, ReasonString,
        UserProperty,
    },
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AuthPacket<'p, const MAX_USER_PROPERTIES: usize> {
    pub reason_code: ReasonCode,

    pub authentication_method: AuthenticationMethod<'p>,
    pub authentication_data: Option<AuthenticationData<'p>>,
    pub reason_string: Option<ReasonString<'p>>,
    pub user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
}

impl<const MAX_USER_PROPERTIES: usize> Packet for AuthPacket<'_, MAX_USER_PROPERTIES> {
    const PACKET_TYPE: PacketType = PacketType::Auth;
}
impl<'p, const MAX_USER_PROPERTIES: usize> RxPacket<'p> for AuthPacket<'p, MAX_USER_PROPERTIES> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding AUTH packet");

        if header.flags() != 0 {
            trace!("invalid AUTH fixed header flags: {}", header.flags());
            return Err(RxError::MalformedPacket);
        }

        let r = &mut reader;

        let authenticate_reason_code = if header.remaining_len.size() == 0 {
            verbose!("received minimal AUTH packet");
            // Despite the specification allowing this, this is a protocol error because the authentication method
            // must always be present and therefore some properties are always present and the abbreviation
            // cannot be taken.
            return Err(RxError::ProtocolError);
        } else {
            verbose!("reading reason code field");
            ReasonCode::read(r).await?
        };

        if !matches!(
            authenticate_reason_code,
            ReasonCode::Success | ReasonCode::ContinueAuthentication | ReasonCode::ReAuthenticate
        ) {
            trace!("invalid AUTH reason code: {:?}", authenticate_reason_code);
            return Err(RxError::ProtocolError);
        }

        verbose!("reading property length field");
        let properties_length = VarByteInt::read(r).await?.size();

        verbose!("property length: {} bytes", properties_length);

        if r.remaining_len() != properties_length {
            trace!("invalid AUTH property length for remaining packet length");
            return Err(RxError::MalformedPacket);
        }

        let mut authentication_method = None;
        let mut authentication_data = None;
        let mut reason_string = None;
        let mut user_properties = Vec::new();

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
            match property_type {
                PropertyType::AuthenticationMethod => authentication_method.try_set(r).await?,
                PropertyType::AuthenticationData => authentication_data.try_set(r).await?,
                PropertyType::ReasonString => reason_string.try_set(r).await?,
                PropertyType::UserProperty if !user_properties.is_full() => {
                    let user_property = UserProperty::read(r).await?;

                    // Safety: `!Vec::is_full` guarantees there is space
                    unsafe { user_properties.push_unchecked(user_property) };
                }
                PropertyType::UserProperty => {
                    UserProperty::skip(r).await?;
                }
                // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                p => {
                    trace!("invalid AUTH property: {:?}", p);
                    return Err(RxError::MalformedPacket);
                }
            };
        }

        let Some(authentication_method) = authentication_method else {
            trace!("authentication method property missing from AUTH packet");
            return Err(RxError::ProtocolError);
        };

        Ok(Self {
            reason_code: authenticate_reason_code,
            authentication_method,
            authentication_data,
            reason_string,
            user_properties,
        })
    }
}
impl<const MAX_USER_PROPERTIES: usize> TxPacket for AuthPacket<'_, MAX_USER_PROPERTIES> {
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, 0x00, self.remaining_len())
            .write(write)
            .await?;

        self.reason_code.write(write).await?;

        let properties_length = self.properties_length();
        properties_length.write(write).await?;

        self.authentication_method.write(write).await?;
        self.authentication_data.write(write).await?;
        self.reason_string.write(write).await?;

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        Ok(())
    }

    fn remaining_len(&self) -> VarByteInt {
        let variable_header_length = wlen!(ReasonCode);

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let total_length = variable_header_length + total_properties_length;

        // max length = MAX_USER_PROPERTIES * 131077 + 196619
        // Invariant: MAX_USER_PROPERTIES <= 2046 => max length <= VarByteInt::MAX_ENCODABLE
        // variable header (reason_code): 1
        // property length: 4
        // properties: MAX_USER_PROPERTIES * 131077 + 196614
        VarByteInt::new_unchecked(total_length as u32)
    }
}

impl<'p, const MAX_USER_PROPERTIES: usize> AuthPacket<'p, MAX_USER_PROPERTIES> {
    pub const fn new(
        reason_code: ReasonCode,
        authentication_method: AuthenticationMethod<'p>,
        authentication_data: Option<AuthenticationData<'p>>,
        reason_string: Option<ReasonString<'p>>,
        user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
    ) -> Self {
        const {
            const_assert!(MAX_USER_PROPERTIES <= 2046);
        }

        Self {
            reason_code,
            authentication_method,
            authentication_data,
            reason_string,
            user_properties,
        }
    }

    fn properties_length(&self) -> VarByteInt {
        let len = self.authentication_method.written_len()
            + self.authentication_data.written_len()
            + self.reason_string.written_len()
            + self
                .user_properties
                .iter()
                .map(Writable::written_len)
                .sum::<usize>();

        // max length = MAX_USER_PROPERTIES * 131077 + 196614
        // Invariant: MAX_USER_PROPERTIES <= 2046 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // authentication method: 65538
        // authentication data: 65538
        // reason string: 65538
        // user properties: MAX_USER_PROPERTIES * 131077
        VarByteInt::new_unchecked(len as u32)
    }
}

#[cfg(test)]
mod unit {
    use heapless::Vec;

    use crate::{
        test::{rx::decode, tx::encode},
        types::{MqttBinary, MqttString, MqttStringPair, ReasonCode},
        v5::{packet::AuthPacket, property::UserProperty},
    };

    #[tokio::test]
    #[test_log::test]
    async fn encode_simple() {
        let packet = AuthPacket::<16>::new(
            ReasonCode::ContinueAuthentication,
            MqttString::from_str("").unwrap().into(),
            None,
            None,
            Vec::new(),
        );

        #[rustfmt::skip]
        encode!(packet, [
            0xF0, //
            0x05, // remaining length
            0x18, // reason code

            0x03, // property length

            // Authentication Method
            0x15, 0x00, 0x00,
        ]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn encode_properties() {
        let packet = AuthPacket::<16>::new(
            ReasonCode::ReAuthenticate,
            MqttString::try_from("SCRAM-SHA-1").unwrap().into(),
            Some(
                MqttBinary::try_from("n,,n=user,r=fyko+d2lbbFgONRv9qkxdawL")
                    .unwrap()
                    .into(),
            ),
            Some(MqttString::try_from("LET ME IN").unwrap().into()),
            [
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("a").unwrap(),
                    MqttString::from_str("b").unwrap(),
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("b").unwrap(),
                    MqttString::from_str("c").unwrap(),
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("c").unwrap(),
                    MqttString::from_str("a").unwrap(),
                )),
            ]
            .into(),
        );

        #[rustfmt::skip]
        encode!(packet, [
                0xF0, //
                0x58, // remaining length
                0x19, // reason code
                0x56, // property length

                // Authentication Method
                0x15, 0x00, 0x0B, b'S', b'C', b'R', b'A', b'M', b'-', b'S', b'H', b'A', b'-', b'1',

                // Authentication Data
                0x16, 0x00, 0x24, b'n', b',', b',', b'n', b'=', b'u', b's', b'e', b'r', b',', b'r', b'=', b'f', b'y', b'k', b'o', b'+',
                b'd', b'2', b'l', b'b', b'b', b'F', b'g', b'O', b'N', b'R', b'v', b'9', b'q', b'k', b'x', b'd', b'a', b'w', b'L',

                // Reason String
                0x1F, 0x00, 0x09, b'L', b'E', b'T', b' ', b'M', b'E', b' ', b'I', b'N',

                0x26,       // User property
                0x00, 0x01, b'a',
                0x00, 0x01, b'b',

                0x26,       // User property
                0x00, 0x01, b'b',
                0x00, 0x01, b'c',

                0x26,       // User property
                0x00, 0x01, b'c',
                0x00, 0x01, b'a',
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_simple() {
        let packet = decode!(
            AuthPacket<16>,
            5,
            [0xF0, 0x05, 0x00, 0x03, 0x15, 0x00, 0x00]
        );

        assert_eq!(packet.reason_code, ReasonCode::Success);
        assert_eq!(
            packet.authentication_method,
            MqttString::from_str("").unwrap().into()
        );
        assert!(packet.authentication_data.is_none());
        assert!(packet.reason_string.is_none());
        assert!(packet.user_properties.is_empty());
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_properties() {
        #[rustfmt::skip]
        let packet = decode!(AuthPacket<16>, 78, [
            0xF0,
            0x4E,
            0x19, // Reason code
            0x4C, // Property length

            // User Property
            0x26, 0x00, 0x03, b'l', b'e', b'd',
                  0x00, 0x08, b'z', b'e', b'p', b'p', b'e', b'l', b'i', b'n',

            // Reason String
            0x1F, 0x00, 0x04, b'g', b'o', b'n', b'e',

            // Authentication Data
            0x16, 0x00, 0x03, b'1', b'.', b'4',

            // User Property
            0x26, 0x00, 0x02, b'A', b'C',
                  0x00, 0x02, b'D', b'C',

            // User Property
            0x26, 0x00, 0x01, b'U',
                  0x00, 0x01, b'2',

            // Authentication Method
            0x15, 0x00, 0x1C, b'b', b'e', b'a', b'r', b'd', b'_', b'l', b'e', b'n', b'g', b't', b'h', b'_',
            b't', b'o', b'_', b'w', b'i', b's', b'd', b'o', b'm', b'_', b'r', b'a', b't', b'i', b'o',
        ]);

        assert_eq!(packet.reason_code, ReasonCode::ReAuthenticate);
        assert_eq!(
            packet.authentication_method,
            MqttString::try_from("beard_length_to_wisdom_ratio")
                .unwrap()
                .into()
        );
        assert_eq!(
            packet.authentication_data,
            Some(MqttBinary::try_from("1.4".as_bytes()).unwrap().into())
        );
        assert_eq!(
            packet.reason_string,
            Some(MqttString::try_from("gone").unwrap().into())
        );
        assert_eq!(
            packet.user_properties.as_slice(),
            &[
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("led").unwrap(),
                    MqttString::from_str("zeppelin").unwrap()
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("AC").unwrap(),
                    MqttString::from_str("DC").unwrap()
                )),
                UserProperty(MqttStringPair::new(
                    MqttString::from_str("U").unwrap(),
                    MqttString::from_str("2").unwrap()
                ))
            ]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn decode_incomplete_user_properties() {
        #[rustfmt::skip]
        let packet = decode!(AuthPacket<1>, 179, [
            0xF0,
            0xB3, 0x01,
            0x00, // Reason code
            0xB0, 0x01, // Property length

            // User Property
            0x26, 0x00, 0x22, b'S', b'h', b'a', b'k', b'e', b' ', b'V', b'e', b'l', b'o', b'c', b'i', b't', b'y', b' ', b'(',
                  b'C', b'e', b'n', b't', b'r', b'i', b'f', b'u', b'g', b'a', b'l', b' ', b'F', b'o', b'r', b'c', b'e', b')',
                  0x00, 0x31, b'4', b'.', b'3', b' ', b'G', b'-', b'F', b'o', b'r', b'c', b'e', b's', b' ', b'(', b'S', b'u', b'f', b'f', b'i', b'c', b'i', b'e', b'n', b't',
                  b' ', b't', b'o', b' ', b'b', b'y', b'p', b'a', b's', b's', b' ', b'a', b'l', b'l', b' ', b'r', b'a', b'i', b'n', b'c', b'o', b'a', b't', b's', b')',

            // User Property
            0x26, 0x00, 0x13, b'F', b'l', b'o', b'o', b'f', b' ', b'D', b'e', b'c', b'o', b'm', b'p', b'r', b'e', b's', b's', b'i', b'o', b'n',
                  0x00, 0x14, b'2', b'0', b'0', b'%', b' ', b'V', b'o', b'l', b'u', b'm', b'e', b' ', b'I', b'n', b'c', b'r', b'e', b'a', b's', b'e',

            // Authentication Method
            0x15, 0x00, 0x00,

            // User Property
            0x26, 0x00, 0x19, b'C', b'o', b'u', b'c', b'h', b' ', b'C', b'o', b'n', b't', b'a', b'm', b'i', b'n', b'a', b't', b'i', b'o', b'n', b' ', b'S', b'p', b'e', b'e', b'd',
                  0x00, 0x0B, b'0', b'.', b'4', b' ', b'S', b'e', b'c', b'o', b'n', b'd', b's',
        ]);

        assert_eq!(
            packet.user_properties.as_slice(),
            &[UserProperty(MqttStringPair::new(
                MqttString::from_str("Shake Velocity (Centrifugal Force)").unwrap(),
                MqttString::from_str("4.3 G-Forces (Sufficient to bypass all raincoats)").unwrap()
            ))]
        );
    }
}
