//! Because PUBACK, PUBREC, PUBREL and PUBCOMP are almost identical we can simplify transcoding and even the structure
//!
//! This helps minimize duplicate code

use core::marker::PhantomData;

use heapless::Vec;

#[cfg(test)]
use crate::types::{MqttString, MqttStringPair};
use crate::{
    buffer::BufferProvider,
    eio::{Read, Write},
    fmt::{trace, verbose},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{PacketIdentifier, ReasonCode, VarByteInt},
    v5::{
        packet::pubacks::types::{Ack, Comp, PubackPacketType, Rec, Rel},
        property::{AtMostOnceProperty, PropertyType, ReasonString, UserProperty},
    },
};

mod types;

pub type PubackPacket<'p, const MAX_USER_PROPERTIES: usize> =
    GenericPubackPacket<'p, Ack, MAX_USER_PROPERTIES>;
pub type PubrecPacket<'p, const MAX_USER_PROPERTIES: usize> =
    GenericPubackPacket<'p, Rec, MAX_USER_PROPERTIES>;
pub type PubrelPacket<'p, const MAX_USER_PROPERTIES: usize> =
    GenericPubackPacket<'p, Rel, MAX_USER_PROPERTIES>;
pub type PubcompPacket<'p, const MAX_USER_PROPERTIES: usize> =
    GenericPubackPacket<'p, Comp, MAX_USER_PROPERTIES>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericPubackPacket<'p, T, const MAX_USER_PROPERTIES: usize> {
    pub packet_identifier: PacketIdentifier,
    pub reason_code: ReasonCode,

    pub reason_string: Option<ReasonString<'p>>,
    pub user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,
    _phantom_data: PhantomData<&'p T>,
}

impl<T: PubackPacketType, const MAX_USER_PROPERTIES: usize> Packet
    for GenericPubackPacket<'_, T, MAX_USER_PROPERTIES>
{
    const PACKET_TYPE: PacketType = T::PACKET_TYPE;
}
impl<'p, T: PubackPacketType, const MAX_USER_PROPERTIES: usize> RxPacket<'p>
    for GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>
{
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding {:?} packet", T::PACKET_TYPE);

        if header.flags() != T::FLAGS {
            trace!(
                "invalid {:?} fixed header flags: {}",
                T::PACKET_TYPE,
                header.flags()
            );
            return Err(RxError::MalformedPacket);
        }

        let r = &mut reader;

        verbose!("reading packet identifier field");
        let packet_identifier = PacketIdentifier::read(r).await?;

        let reason_code = if header.remaining_len.size() == 2 {
            ReasonCode::Success
        } else {
            verbose!("reading reason code field");
            let c = ReasonCode::read(r).await?;
            if !T::reason_code_allowed(c) {
                trace!("invalid {:?} reason code: {:?}", T::PACKET_TYPE, c);
                return Err(RxError::ProtocolError);
            }
            c
        };

        let properties_length = if header.remaining_len.value() < 4 {
            0
        } else {
            verbose!("reading property length field");
            VarByteInt::read(r).await?.size()
        };

        verbose!("property length: {} bytes", properties_length);

        if r.remaining_len() != properties_length {
            trace!(
                "invalid {:?} property length for remaining packet length",
                T::PACKET_TYPE
            );
            return Err(RxError::MalformedPacket);
        }

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
                PropertyType::ReasonString => reason_string.try_set(r).await?,
                PropertyType::UserProperty if !user_properties.is_full() => {
                    let user_property = UserProperty::read(r).await?;

                    // Safety: `!Vec::is_full` guarantees there is space
                    unsafe { user_properties.push_unchecked(user_property) };
                }
                PropertyType::UserProperty => {
                    UserProperty::skip(r).await?;
                }
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    trace!("invalid packet {:?} property: {:?}", T::PACKET_TYPE, p);
                    return Err(RxError::MalformedPacket);
                }
            };
        }

        Ok(Self {
            packet_identifier,
            reason_code,
            reason_string,
            user_properties,
            _phantom_data: PhantomData,
        })
    }
}
impl<T: PubackPacketType, const MAX_USER_PROPERTIES: usize> TxPacket
    for GenericPubackPacket<'_, T, MAX_USER_PROPERTIES>
{
    fn remaining_len(&self) -> VarByteInt {
        let variable_header_length = self.packet_identifier.written_len() + wlen!(ReasonCode);

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let total_length = variable_header_length + total_properties_length;

        // max length = MAX_USER_PROPERTIES * 131077 + 65545
        // Invariant: MAX_USER_PROPERTIES <= 2047 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // variable header: 3
        // property length: 4
        // reason string: 65538
        // user properties: MAX_USER_PROPERTIES * 131077
        VarByteInt::new_unchecked(total_length as u32)
    }

    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, T::FLAGS, self.remaining_len())
            .write(write)
            .await?;

        self.packet_identifier.write(write).await?;
        self.reason_code.write(write).await?;

        let properties_length = self.properties_length();
        properties_length.write(write).await?;

        self.reason_string.write(write).await?;

        for user_property in &self.user_properties {
            user_property.write(write).await?;
        }

        Ok(())
    }
}

impl<'p, T: PubackPacketType, const MAX_USER_PROPERTIES: usize>
    GenericPubackPacket<'p, T, MAX_USER_PROPERTIES>
{
    pub const fn new(packet_identifier: PacketIdentifier, reason_code: ReasonCode) -> Self {
        Self {
            packet_identifier,
            reason_code,
            reason_string: None,
            user_properties: Vec::new(),
            _phantom_data: PhantomData,
        }
    }

    #[cfg(test)]
    pub fn add_reason_string(&mut self, reason_string: MqttString<'p>) {
        self.reason_string = Some(reason_string.into());
    }
    #[cfg(test)]
    pub fn add_user_property(&mut self, user_property: MqttStringPair<'p>) {
        self.user_properties
            .push(UserProperty(user_property))
            .unwrap();
    }

    fn properties_length(&self) -> VarByteInt {
        let len = self.reason_string.written_len()
            + self
                .user_properties
                .iter()
                .map(Writable::written_len)
                .sum::<usize>();

        // max length = MAX_USER_PROPERTIES * 131077 + 65538
        // Invariant: MAX_USER_PROPERTIES <= 2047 => max length <= VarByteInt::MAX_ENCODABLE
        //
        // reason string: 65538
        // user properties: MAX_USER_PROPERTIES * 131077
        VarByteInt::new_unchecked(len as u32)
    }
}

#[cfg(test)]
mod unit {
    mod ack {
        use core::num::NonZero;

        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, MqttStringPair, PacketIdentifier, ReasonCode},
            v5::{
                packet::PubackPacket,
                property::{ReasonString, UserProperty},
            },
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubackPacket::<0>::new(PacketIdentifier::new(NonZero::new(7439).unwrap()), ReasonCode::NotAuthorized),
                [
                    0x40,
                    0x04,
                    0x1D, // Packet identifier MSB
                    0x0F, // Packet identifier LSB
                    0x87, // Reason Code
                    0x00, // Property length
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn encode_properties() {
            let mut packet = PubackPacket::<16>::new(
                PacketIdentifier::new(NonZero::new(23485).unwrap()),
                ReasonCode::TopicNameInvalid,
            );
            packet.add_reason_string(MqttString::from_str("invalid topic name!").unwrap());
            packet.add_user_property(MqttStringPair::new(
                MqttString::from_str("topic name").unwrap(),
                MqttString::from_str("invalid").unwrap(),
            ));
            packet.add_user_property(MqttStringPair::new(
                MqttString::from_str("accepted").unwrap(),
                MqttString::from_str("negative").unwrap(),
            ));

            #[rustfmt::skip]
            encode!(
                packet,
                [
                    0x40,
                    0x45,
                    0x5B, // Packet identifier MSB
                    0xBD, // Packet identifier LSB
                    0x90, // Reason Code
                    0x41, // Property length
                    // Reason String
                    0x1F, 0x00, 0x13, b'i', b'n', b'v', b'a', b'l', b'i', b'd', b' ', b't', b'o', b'p', b'i', b'c', b' ', b'n', b'a', b'm', b'e', b'!',
                    // User Property
                    0x26, 0x00, 0x0A, b't', b'o', b'p', b'i', b'c', b' ', b'n', b'a', b'm', b'e',
                          0x00, 0x07, b'i', b'n', b'v', b'a', b'l', b'i', b'd',
                    // User Property
                    0x26, 0x00, 0x08, b'a', b'c', b'c', b'e', b'p', b't', b'e', b'd',
                          0x00, 0x08, b'n', b'e', b'g', b'a', b't', b'i', b'v', b'e',
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_simple() {
            let packet = decode!(PubackPacket<2>, 4, [0x40, 0x04, 0x26, 0x29, 0x10, 0x00]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9769).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::NoMatchingSubscribers);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubackPacket<2>, 3, [0x40, 0x03, 0x71, 0x59, 0x80]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(29017).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::UnspecifiedError);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubackPacket<2>, 2, [0x40, 0x02, 0x89, 0x35]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(35125).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubackPacket<2>, 42, [
                0x40, 0x2A,
                0x12, 0x34, // Packet Identifier
                0x99, // Reason Code
                0x26, // Property length
                // Reason String
                0x1F, 0x00, 0x0B, b't', b'e', b's', b't', b' ', b'r', b'e', b'a', b's', b'o', b'n',
                // User Property
                0x26, 0x00, 0x09, b't', b'e', b's', b't', b'-', b'n', b'a', b'm', b'e', 0x00, 0x0A,
                b't', b'e', b's', b't', b'-', b'v', b'a', b'l', b'u', b'e',
            ]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(4660).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::PayloadFormatInvalid);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );

            assert_eq!(packet.user_properties.len(), 1);
            assert_eq!(
                packet.user_properties.first().unwrap(),
                &UserProperty(MqttStringPair::new(
                    MqttString::try_from("test-name").unwrap(),
                    MqttString::try_from("test-value").unwrap()
                ))
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_incomplete_user_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubackPacket<2>, 31, [
                0x40, 0x1F,
                0x12, 0x34, // Packet Identifier
                0x00,       // Reason Code
                0x1B,       // Property length

                // User Property
                0x26, 0x00, 0x02, b'k', b'1',
                      0x00, 0x02, b'v', b'1',

                // User Property
                0x26, 0x00, 0x02, b'k', b'2',
                      0x00, 0x02, b'v', b'2',

                // User Property
                0x26, 0x00, 0x02, b'k', b'3',
                      0x00, 0x02, b'v', b'3',
            ]);

            assert_eq!(
                packet.user_properties.as_slice(),
                &[
                    UserProperty(MqttStringPair::new(
                        MqttString::try_from("k1").unwrap(),
                        MqttString::try_from("v1").unwrap()
                    )),
                    UserProperty(MqttStringPair::new(
                        MqttString::try_from("k2").unwrap(),
                        MqttString::try_from("v2").unwrap()
                    ))
                ]
            );
        }
    }

    mod rec {
        use core::num::NonZero;

        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, MqttStringPair, PacketIdentifier, ReasonCode},
            v5::{
                packet::PubrecPacket,
                property::{ReasonString, UserProperty},
            },
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubrecPacket::<0>::new(PacketIdentifier::new(NonZero::new(876).unwrap()), ReasonCode::QuotaExceeded),
                [
                    0x50,
                    0x04,
                    0x03, // Packet identifier MSB
                    0x6C, // Packet identifier LSB
                    0x97, // Reason Code
                    0x00, // Property length
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn encode_properties() {
            let mut packet = PubrecPacket::<16>::new(
                PacketIdentifier::new(NonZero::new(23895).unwrap()),
                ReasonCode::ImplementationSpecificError,
            );
            packet.add_reason_string(MqttString::from_str("I crashed :(").unwrap());
            packet.add_user_property(MqttStringPair::new(
                MqttString::from_str("me").unwrap(),
                MqttString::from_str("somewhere over the rainbow").unwrap(),
            ));
            packet.add_user_property(MqttStringPair::new(
                MqttString::from_str("problem").unwrap(),
                MqttString::from_str("yours").unwrap(),
            ));

            #[rustfmt::skip]
            encode!(
                packet,
                [
                    0x50,
                    0x45,
                    0x5D, // Packet identifier MSB
                    0x57, // Packet identifier LSB
                    0x83, // Reason Code
                    0x41, // Property length
                    // Reason String
                    0x1F, 0x00, 0x0C, b'I', b' ', b'c', b'r', b'a', b's', b'h', b'e', b'd', b' ', b':', b'(',
                    // User Property
                    0x26, 0x00, 0x02, b'm', b'e',
                          0x00, 0x1A, b's', b'o', b'm', b'e', b'w', b'h', b'e', b'r', b'e', b' ', b'o', b'v', b'e', b'r', b' ', b't', b'h', b'e', b' ', b'r', b'a', b'i', b'n', b'b', b'o', b'w',
                    // User Property
                    0x26, 0x00, 0x07, b'p', b'r', b'o', b'b', b'l', b'e', b'm',
                          0x00, 0x05, b'y', b'o', b'u', b'r', b's',
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_simple() {
            let packet = decode!(PubrecPacket<2>, 4, [0x50, 0x04, 0x26, 0x94, 0x91, 0x00]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9876).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierInUse);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubrecPacket<2>, 3, [0x50, 0x03, 0x45, 0xC9, 0x83]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(17865).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::ImplementationSpecificError);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubrecPacket<2>, 2, [0x50, 0x02, 0x5B, 0xBF]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(23487).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubrecPacket<2>, 42, [
                0x50,
                0x2A,
                0x26, 0x3A, // Packet Identifier
                0x90,       // Reason Code
                0x26,       // Property length

                // Reason String
                0x1F, 0x00, 0x0B, b't', b'e', b's', b't', b' ', b'r', b'e', b'a', b's', b'o', b'n',

                // User Property
                0x26, 0x00, 0x09, b't', b'e', b's', b't', b'-', b'n', b'a', b'm', b'e',
                      0x00, 0x0A, b't', b'e', b's', b't', b'-', b'v', b'a', b'l', b'u', b'e',
            ]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9786).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::TopicNameInvalid);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );

            assert_eq!(packet.user_properties.len(), 1);
            assert_eq!(
                packet.user_properties.first().unwrap(),
                &UserProperty(MqttStringPair::new(
                    MqttString::try_from("test-name").unwrap(),
                    MqttString::try_from("test-value").unwrap()
                ))
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_incomplete_user_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubrecPacket<1>, 31, [
                0x50, 0x1F,
                0x12, 0x34, // Packet Identifier
                0x00,       // Reason Code
                0x1B,       // Property length

                // User Property
                0x26, 0x00, 0x02, b'k', b'1',
                      0x00, 0x02, b'v', b'1',

                // User Property
                0x26, 0x00, 0x02, b'k', b'2',
                      0x00, 0x02, b'v', b'2',

                // User Property
                0x26, 0x00, 0x02, b'k', b'3',
                      0x00, 0x02, b'v', b'3',
            ]);

            assert_eq!(
                packet.user_properties.as_slice(),
                &[UserProperty(MqttStringPair::new(
                    MqttString::try_from("k1").unwrap(),
                    MqttString::try_from("v1").unwrap()
                ))]
            );
        }
    }

    mod rel {
        use core::num::NonZero;

        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, MqttStringPair, PacketIdentifier, ReasonCode},
            v5::{
                packet::PubrelPacket,
                property::{ReasonString, UserProperty},
            },
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubrelPacket::<0>::new(PacketIdentifier::new(NonZero::new(876).unwrap()), ReasonCode::PacketIdentifierNotFound),
                [
                    0x62,
                    0x04,
                    0x03, // Packet identifier MSB
                    0x6C, // Packet identifier LSB
                    0x92, // Reason Code
                    0x00, // Property length
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn encode_properties() {
            let mut packet = PubrelPacket::<16>::new(
                PacketIdentifier::new(NonZero::new(9786).unwrap()),
                ReasonCode::PacketIdentifierNotFound,
            );
            packet.add_reason_string(MqttString::try_from("test reason").unwrap());
            packet.add_user_property(MqttStringPair::new(
                MqttString::try_from("test-name").unwrap(),
                MqttString::try_from("test-value").unwrap(),
            ));

            #[rustfmt::skip]
            encode!(
                packet,
                [
                    0x62,
                    0x2A,
                    0x26,   // Packet identifier MSB
                    0x3A,   // Packet identifier LSB
                    0x92,   // Reason Code
                    0x26,   // Property length
                    // Reason String
                    0x1F, 0x00, 0x0B, b't', b'e', b's', b't', b' ', b'r', b'e', b'a', b's', b'o', b'n',
                    // User Property
                    0x26, 0x00, 0x09, b't', b'e', b's', b't', b'-', b'n', b'a', b'm', b'e',
                          0x00, 0x0A, b't', b'e', b's', b't', b'-', b'v', b'a', b'l', b'u', b'e',
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_simple() {
            let packet = decode!(PubrelPacket<2>, 4, [0x62, 0x04, 0x26, 0x94, 0x00, 0x00]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9876).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubrelPacket<2>, 3, [0x62, 0x03, 0x45, 0xC9, 0x92]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(17865).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubrelPacket<2>, 2, [0x62, 0x02, 0x5B, 0xBF]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(23487).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(
                PubrelPacket<2>,
                42,
                [
                    0x62,
                    0x2A, 

                    0x26, 0x3A, // Packet Identifier
                    0x92,       // Reason Code

                    0x26,       // Property length

                    // Reason String
                    0x1F, 0x00, 0x0B,
                          b't', b'e', b's', b't', b' ', b'r', b'e', b'a', b's', b'o', b'n', 

                    // User Property
                    0x26, 0x00, 0x09, b't', b'e', b's', b't', b'-', b'n', b'a', b'm', b'e', 
                          0x00, 0x0A, b't', b'e', b's', b't', b'-', b'v', b'a', b'l', b'u', b'e',
                ]
            );

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9786).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );

            assert_eq!(packet.user_properties.len(), 1);
            assert_eq!(
                packet.user_properties.first().unwrap(),
                &UserProperty(MqttStringPair::new(
                    MqttString::try_from("test-name").unwrap(),
                    MqttString::try_from("test-value").unwrap()
                ))
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_incomplete_user_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubrelPacket<1>, 31, [
                0x62, 0x1F,
                0x12, 0x34, // Packet Identifier
                0x00,       // Reason Code
                0x1B,       // Property length

                // User Property
                0x26, 0x00, 0x02, b'k', b'1',
                      0x00, 0x02, b'v', b'1',

                // User Property
                0x26, 0x00, 0x02, b'k', b'2',
                      0x00, 0x02, b'v', b'2',

                // User Property
                0x26, 0x00, 0x02, b'k', b'3',
                      0x00, 0x02, b'v', b'3',
            ]);

            assert_eq!(
                packet.user_properties.as_slice(),
                &[UserProperty(MqttStringPair::new(
                    MqttString::try_from("k1").unwrap(),
                    MqttString::try_from("v1").unwrap()
                ))]
            );
        }
    }

    mod comp {
        use core::num::NonZero;

        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, MqttStringPair, PacketIdentifier, ReasonCode},
            v5::{
                packet::PubcompPacket,
                property::{ReasonString, UserProperty},
            },
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubcompPacket::<0>::new(PacketIdentifier::new(NonZero::new(876).unwrap()), ReasonCode::PacketIdentifierNotFound),
                [
                    0x70,
                    0x04,
                    0x03, // Packet identifier MSB
                    0x6C, // Packet identifier LSB
                    0x92, // Reason Code
                    0x00, // Property length
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn encode_properties() {
            let mut packet = PubcompPacket::<16>::new(
                PacketIdentifier::new(NonZero::new(9786).unwrap()),
                ReasonCode::PacketIdentifierNotFound,
            );
            packet.add_reason_string(MqttString::try_from("test reason").unwrap());
            packet.add_user_property(MqttStringPair::new(
                MqttString::try_from("test-name").unwrap(),
                MqttString::try_from("test-value").unwrap(),
            ));

            #[rustfmt::skip]
            encode!(
                packet,
                [
                    0x70,
                    0x2A,
                    0x26,   // Packet identifier MSB
                    0x3A,   // Packet identifier LSB
                    0x92,   // Reason Code
                    0x26,   // Property length

                    // Reason String
                    0x1F, 0x00, 0x0B, b't', b'e', b's', b't', b' ', b'r', b'e', b'a', b's', b'o', b'n',

                    // User Property
                    0x26, 0x00, 0x09, b't', b'e', b's', b't', b'-', b'n', b'a', b'm', b'e',
                          0x00, 0x0A, b't', b'e', b's', b't', b'-', b'v', b'a', b'l', b'u', b'e',
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_simple() {
            let packet = decode!(PubcompPacket<2>, 4, [0x70, 0x04, 0x26, 0x94, 0x00, 0x00]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9876).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubcompPacket<2>, 3, [0x70, 0x03, 0x45, 0xC9, 0x92]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(17865).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubcompPacket<2>, 2, [0x70, 0x02, 0x5B, 0xBF]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(23487).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubcompPacket<2>, 42, [
                0x70,
                0x2A,

                0x26, 0x3A, // Packet Identifier
                0x92,       // Reason Code
                0x26,       // Property length

                // Reason String
                0x1F, 0x00, 0x0B, b't', b'e', b's', b't', b' ', b'r', b'e', b'a', b's', b'o', b'n',

                // User Property
                0x26, 0x00, 0x09, b't', b'e', b's', b't', b'-', b'n', b'a', b'm', b'e',
                      0x00, 0x0A, b't', b'e', b's', b't', b'-', b'v', b'a', b'l', b'u', b'e',
            ]);

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9786).unwrap())
            );
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );

            assert_eq!(packet.user_properties.len(), 1);
            assert_eq!(
                packet.user_properties.first().unwrap(),
                &UserProperty(MqttStringPair::new(
                    MqttString::try_from("test-name").unwrap(),
                    MqttString::try_from("test-value").unwrap()
                ))
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_incomplete_user_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubcompPacket<2>, 31, [
                0x70, 0x1F,
                0x12, 0x34, // Packet Identifier
                0x00,       // Reason Code
                0x1B,       // Property length

                // User Property
                0x26, 0x00, 0x02, b'k', b'1', 
                      0x00, 0x02, b'v', b'1',

                // User Property
                0x26, 0x00, 0x02, b'k', b'2', 
                      0x00, 0x02, b'v', b'2',

                // User Property
                0x26, 0x00, 0x02, b'k', b'3', 
                      0x00, 0x02, b'v', b'3',
            ]);

            assert_eq!(
                packet.user_properties.as_slice(),
                &[
                    UserProperty(MqttStringPair::new(
                        MqttString::try_from("k1").unwrap(),
                        MqttString::try_from("v1").unwrap()
                    )),
                    UserProperty(MqttStringPair::new(
                        MqttString::try_from("k2").unwrap(),
                        MqttString::try_from("v2").unwrap()
                    ))
                ]
            );
        }
    }
}
