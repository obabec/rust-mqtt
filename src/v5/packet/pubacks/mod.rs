//! Because PUBACK, PUBREC, PUBREL and PUBCOMP are almost identical we can simplify transcoding and even the structure
//!
//! This helps minimize duplicate code

use core::marker::PhantomData;

use crate::{
    buffer::BufferProvider,
    eio::{Read, Write},
    fmt::{error, trace},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::{ReasonCode, VarByteInt},
    v5::{
        packet::pubacks::types::{Ack, Comp, PubackPacketType, Rec, Rel},
        property::{AtMostOnceProperty, PropertyType, ReasonString},
    },
};

mod types;

pub type PubackPacket<'p> = GenericPubackPacket<'p, Ack>;
pub type PubrecPacket<'p> = GenericPubackPacket<'p, Rec>;
pub type PubrelPacket<'p> = GenericPubackPacket<'p, Rel>;
pub type PubcompPacket<'p> = GenericPubackPacket<'p, Comp>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericPubackPacket<'p, T: PubackPacketType> {
    pub packet_identifier: u16,
    pub reason_code: ReasonCode,
    pub reason_string: Option<ReasonString<'p>>,
    _phantom_data: PhantomData<T>,
}

impl<'p, T: PubackPacketType> Packet for GenericPubackPacket<'p, T> {
    const PACKET_TYPE: PacketType = T::PACKET_TYPE;
}
impl<'p, T: PubackPacketType> RxPacket<'p> for GenericPubackPacket<'p, T> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding");

        if header.flags() != T::FLAGS {
            error!("flags are not matching");
            return Err(RxError::MalformedPacket);
        }

        let r = &mut reader;

        trace!("reading packet identifier");
        let packet_identifier = u16::read(r).await?;

        let reason_code = if header.remaining_len.size() == 2 {
            ReasonCode::Success
        } else {
            trace!("reading reason code");
            let c = ReasonCode::read(r).await?;
            if !T::reason_code_allowed(c) {
                error!("invalid reason code: {:?}", c);
                return Err(RxError::ProtocolError);
            }
            c
        };

        let mut reason_string = None;

        let properties_length = if header.remaining_len.value() < 4 {
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
                PropertyType::ReasonString => reason_string.try_set(r).await?,
                PropertyType::UserProperty => {
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                },
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    error!("packet contains unexpected property {:?}", p);
                    return Err(RxError::MalformedPacket)
                },
            };
        }

        Ok(Self {
            packet_identifier,
            reason_code,
            reason_string,
            _phantom_data: PhantomData,
        })
    }
}
impl<'p, T: PubackPacketType> TxPacket for GenericPubackPacket<'p, T> {
    fn remaining_len(&self) -> VarByteInt {
        let variable_header_length = wlen!(u16) + wlen!(ReasonCode);

        let properties_length = self.properties_length();
        let total_properties_length = properties_length.size() + properties_length.written_len();

        let total_length = variable_header_length + total_properties_length;

        // Invariant: Max length = 65545 < VarByteInt::MAX_ENCODABLE
        // properties length: 4
        // properties: 65538
        // variable header: 3
        VarByteInt::new(total_length as u32)
    }

    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        FixedHeader::new(Self::PACKET_TYPE, T::FLAGS, self.remaining_len())
            .write(write)
            .await?;

        self.packet_identifier.write(write).await?;
        self.reason_code.write(write).await?;
        match &self.reason_string {
            // Invariant: reason string length 65537 < VarByteInt::MAX_ENCODABLE
            Some(r) => {
                VarByteInt::new(r.written_len() as u32).write(write).await?;
                r.write(write).await?;
            }
            // Invariant: 0 < VarByteInt::MAX_ENCODABLE
            None => VarByteInt::new(0).write(write).await?,
        }

        Ok(())
    }
}

impl<'p, T: PubackPacketType> GenericPubackPacket<'p, T> {
    pub const fn new(packet_identifier: u16, reason_code: ReasonCode) -> Self {
        Self {
            packet_identifier,
            reason_code,
            reason_string: None,
            _phantom_data: PhantomData,
        }
    }

    fn properties_length(&self) -> VarByteInt {
        let len = self.reason_string.written_len();

        // Invariant: Max length of reason string is 65538 < VarByteInt::MAX_ENCODABLE
        VarByteInt::new(len as u32)
    }
}

#[cfg(test)]
mod unit {
    mod ack {
        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, ReasonCode},
            v5::{packet::PubackPacket, property::ReasonString},
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubackPacket::new(7439, ReasonCode::NotAuthorized),
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
        async fn decode_simple() {
            let packet = decode!(PubackPacket, 4, [0x40, 0x04, 0x26, 0x29, 0x10, 0x00]);

            assert_eq!(packet.packet_identifier, 9769);
            assert_eq!(packet.reason_code, ReasonCode::NoMatchingSubscribers);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubackPacket, 3, [0x40, 0x03, 0x71, 0x59, 0x80]);

            assert_eq!(packet.packet_identifier, 29017);
            assert_eq!(packet.reason_code, ReasonCode::UnspecifiedError);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubackPacket, 2, [0x40, 0x02, 0x89, 0x35]);

            assert_eq!(packet.packet_identifier, 35125);
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubackPacket, 42, [
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

            assert_eq!(packet.packet_identifier, 4660);
            assert_eq!(packet.reason_code, ReasonCode::PayloadFormatInvalid);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );
        }
    }

    mod rec {
        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, ReasonCode},
            v5::{packet::PubrecPacket, property::ReasonString},
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubrecPacket::new(876, ReasonCode::QuotaExceeded),
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
        async fn decode_simple() {
            let packet = decode!(PubrecPacket, 4, [0x50, 0x04, 0x26, 0x94, 0x91, 0x00]);

            assert_eq!(packet.packet_identifier, 9876);
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierInUse);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubrecPacket, 3, [0x50, 0x03, 0x45, 0xC9, 0x83]);

            assert_eq!(packet.packet_identifier, 17865);
            assert_eq!(packet.reason_code, ReasonCode::ImplementationSpecificError);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubrecPacket, 2, [0x50, 0x02, 0x5B, 0xBF]);

            assert_eq!(packet.packet_identifier, 23487);
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubrecPacket, 42, [
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

            assert_eq!(packet.packet_identifier, 9786);
            assert_eq!(packet.reason_code, ReasonCode::TopicNameInvalid);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );
        }
    }

    mod rel {
        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, ReasonCode},
            v5::{packet::PubrelPacket, property::ReasonString},
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubrelPacket::new(876, ReasonCode::PacketIdentifierNotFound),
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
        async fn decode_simple() {
            let packet = decode!(PubrelPacket, 4, [0x62, 0x04, 0x26, 0x94, 0x00, 0x00]);

            assert_eq!(packet.packet_identifier, 9876);
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubrelPacket, 3, [0x62, 0x03, 0x45, 0xC9, 0x92]);

            assert_eq!(packet.packet_identifier, 17865);
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubrelPacket, 2, [0x62, 0x02, 0x5B, 0xBF]);

            assert_eq!(packet.packet_identifier, 23487);
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(
                PubrelPacket,
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

            assert_eq!(packet.packet_identifier, 9786);
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );
        }
    }

    mod comp {
        use crate::{
            test::{rx::decode, tx::encode},
            types::{MqttString, ReasonCode},
            v5::{packet::PubcompPacket, property::ReasonString},
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode_simple() {
            #[rustfmt::skip]
            encode!(
                PubcompPacket::new(876, ReasonCode::PacketIdentifierNotFound),
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
        async fn decode_simple() {
            let packet = decode!(PubcompPacket, 4, [0x70, 0x04, 0x26, 0x94, 0x00, 0x00]);

            assert_eq!(packet.packet_identifier, 9876);
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_abbreviated() {
            let packet = decode!(PubcompPacket, 3, [0x70, 0x03, 0x45, 0xC9, 0x92]);

            assert_eq!(packet.packet_identifier, 17865);
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_minimal() {
            let packet = decode!(PubcompPacket, 2, [0x70, 0x02, 0x5B, 0xBF]);

            assert_eq!(packet.packet_identifier, 23487);
            assert_eq!(packet.reason_code, ReasonCode::Success);
            assert!(packet.reason_string.is_none());
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(PubcompPacket, 42, [
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

            assert_eq!(packet.packet_identifier, 9786);
            assert_eq!(packet.reason_code, ReasonCode::PacketIdentifierNotFound);
            assert_eq!(
                packet.reason_string,
                Some(ReasonString(MqttString::try_from("test reason").unwrap()))
            );
        }
    }
}
