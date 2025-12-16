use core::marker::PhantomData;

use heapless::Vec;

use crate::{
    buffer::BufferProvider,
    eio::Read,
    fmt::{error, trace},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket},
    types::{ReasonCode, VarByteInt},
    v5::{
        packet::subacks::types::{Suback, SubackPacketType, Unsuback},
        property::PropertyType,
    },
};

mod types;

pub type SubackPacket<'p, const MAX_TOPIC_FILTERS: usize> =
    GenericSubackPacket<'p, Suback, MAX_TOPIC_FILTERS>;
pub type UnsubackPacket<'p, const MAX_TOPIC_FILTERS: usize> =
    GenericSubackPacket<'p, Unsuback, MAX_TOPIC_FILTERS>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericSubackPacket<'p, T: SubackPacketType, const MAX_TOPIC_FILTERS: usize> {
    pub packet_identifier: u16,
    // reason string is currently unused and does not have to be read into memory.
    // reason_string: Option<ReasonString<'p>>,
    pub reason_codes: Vec<ReasonCode, MAX_TOPIC_FILTERS>,
    _phantom_data: PhantomData<&'p T>,
}

impl<'p, T: SubackPacketType, const MAX_TOPIC_FILTERS: usize> Packet
    for GenericSubackPacket<'p, T, MAX_TOPIC_FILTERS>
{
    const PACKET_TYPE: PacketType = T::PACKET_TYPE;
}
impl<'p, T: SubackPacketType, const MAX_TOPIC_FILTERS: usize> RxPacket<'p>
    for GenericSubackPacket<'static, T, MAX_TOPIC_FILTERS>
{
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

        trace!("reading packet identifier");
        let packet_identifier = u16::read(r).await?;

        trace!("reading properties length");
        let mut properties_length = VarByteInt::read(r).await?.size();

        trace!("properties length = {}", properties_length);

        if properties_length > r.remaining_len() {
            error!("properties length greater than remaining length");
            return Err(RxError::MalformedPacket);
        }
        if properties_length == r.remaining_len() {
            error!("packet contains no reason code");
            return Err(RxError::ProtocolError);
        }

        while properties_length > 0 {
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

            let mut seen_reason_string = false;

            #[rustfmt::skip]
            match property_type {
                PropertyType::ReasonString if seen_reason_string => {
                    return Err(RxError::MalformedPacket);
                }
                PropertyType::ReasonString => {
                    properties_length = properties_length.checked_sub(1).ok_or(RxError::MalformedPacket)?;
                    seen_reason_string = true;
                    let len = u16::read(r).await? as usize;
                    trace!("Skipping reason string of length {}", len);
                    r.skip(len).await?;
                    properties_length = properties_length.checked_sub(wlen!(u16) + len).ok_or(RxError::MalformedPacket)?;
                },
                PropertyType::UserProperty => {
                    properties_length = properties_length.checked_sub(1).ok_or(RxError::MalformedPacket)?;
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    properties_length = properties_length.checked_sub(wlen!(u16) + len).ok_or(RxError::MalformedPacket)?;
                    let len = u16::read(r).await? as usize;
                    r.skip(len).await?;
                    properties_length = properties_length.checked_sub(wlen!(u16) + len).ok_or(RxError::MalformedPacket)?;
                },
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    error!("packet contains unexpected property {:?}", p);
                    return Err(RxError::MalformedPacket)
                },
            };

            let _ = seen_reason_string;
        }

        let mut reason_codes = Vec::new();
        while r.remaining_len() > 0 {
            trace!("reading reason code");
            let reason_code = ReasonCode::read(r).await?;

            if !T::reason_code_allowed(reason_code) {
                error!("invalid reason code: {:?}", reason_code);
                return Err(RxError::ProtocolError);
            }
            reason_codes
                .push(reason_code)
                .map_err(|_| RxError::InsufficientConstSpace)?;
        }

        let packet = Self {
            packet_identifier,
            reason_codes,
            _phantom_data: PhantomData,
        };

        Ok(packet)
    }
}

#[cfg(test)]
mod unit {
    mod suback {
        use heapless::Vec;

        use crate::{test::rx::decode, types::ReasonCode, v5::packet::SubackPacket};

        #[tokio::test]
        #[test_log::test]
        async fn decode_payload() {
            #[rustfmt::skip]
            let packet: SubackPacket<'_, 12> = decode!(
                SubackPacket<12>,
                15,
                [
                    0x90,
                    0x0F,

                    0x17, 0x89,
                    0x00,

                    // Reason codes
                    0x00, 0xA2, 0x01, 0xA1, 0x02, 0x9E, 0x80, 0x97, 0x83, 0x91, 0x87, 0x8F,
                ]
            );

            assert_eq!(packet.packet_identifier, 6025);
            // assert!(packet.reason_string.is_none());

            let mut reason_codes: Vec<_, 12> = Vec::new();
            reason_codes.push(ReasonCode::Success).unwrap();
            reason_codes
                .push(ReasonCode::WildcardSubscriptionsNotSupported)
                .unwrap();
            reason_codes.push(ReasonCode::GrantedQoS1).unwrap();
            reason_codes
                .push(ReasonCode::SubscriptionIdentifiersNotSupported)
                .unwrap();
            reason_codes.push(ReasonCode::GrantedQoS2).unwrap();
            reason_codes
                .push(ReasonCode::SharedSubscriptionsNotSupported)
                .unwrap();
            reason_codes.push(ReasonCode::UnspecifiedError).unwrap();
            reason_codes.push(ReasonCode::QuotaExceeded).unwrap();
            reason_codes
                .push(ReasonCode::ImplementationSpecificError)
                .unwrap();
            reason_codes
                .push(ReasonCode::PacketIdentifierInUse)
                .unwrap();
            reason_codes.push(ReasonCode::NotAuthorized).unwrap();
            reason_codes.push(ReasonCode::TopicFilterInvalid).unwrap();
            assert_eq!(packet.reason_codes, reason_codes);
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet: SubackPacket<'_, 1> = decode!(
                SubackPacket<1>,
                61,
                [
                    0x90,
                    0x3D, // remaining length

                    0x15, 0xF4, // packet identifier
                    
                    0x39, // Property length

                    0x1F, 0x00, 0x0C, b'c', b'r', b'a', b'z', b'y', b' ', b't', b'h', b'i', b'n', b'g', b's',

                    0x26, 0x00, 0x09, b's', b'o', b'm', b'e', b' ', b'n', b'a', b'm', b'e',
                        0x00, 0x09, b'a', b'n', b'y', b' ', b'v', b'a', b'l', b'u', b'e',

                    0x26, 0x00, 0x07, b'a', b'n', b'y', b' ', b'k', b'e', b'y',
                        0x00, 0x07, b'a', b' ', b'v', b'a', b'l', b'u', b'e',

                    // Reason codes
                    0x00,
                ]
            );

            assert_eq!(packet.packet_identifier, 5620);
            // assert_eq!(
            //     packet.reason_string,
            //     Some(ReasonString(MqttString::try_from("crazy things").unwrap()))
            // );

            let mut reason_codes: Vec<_, 1> = Vec::new();
            reason_codes.push(ReasonCode::Success).unwrap();
            assert_eq!(packet.reason_codes, reason_codes);
        }
    }

    mod unsuback {
        use heapless::Vec;

        use crate::{test::rx::decode, types::ReasonCode, v5::packet::UnsubackPacket};

        #[tokio::test]
        #[test_log::test]
        async fn decode_payload() {
            #[rustfmt::skip]
            let packet: UnsubackPacket<'_, 7> = decode!(
                UnsubackPacket<7>,
                10,
                [
                    0xB0,
                    0x0A,

                    0xA3, 0xF4, 0x00, 
                    
                    // Reason codes
                    0x00, 0x91, 0x11, 0x8F, 0x80, 0x87, 0x83,
                ]
            );

            assert_eq!(packet.packet_identifier, 41972);
            // assert!(packet.reason_string.is_none());

            let mut reason_codes: Vec<_, 7> = Vec::new();

            reason_codes.push(ReasonCode::Success).unwrap();
            reason_codes
                .push(ReasonCode::PacketIdentifierInUse)
                .unwrap();
            reason_codes
                .push(ReasonCode::NoSubscriptionExisted)
                .unwrap();
            reason_codes.push(ReasonCode::TopicFilterInvalid).unwrap();
            reason_codes.push(ReasonCode::UnspecifiedError).unwrap();
            reason_codes.push(ReasonCode::NotAuthorized).unwrap();
            reason_codes
                .push(ReasonCode::ImplementationSpecificError)
                .unwrap();

            assert_eq!(packet.reason_codes, reason_codes);
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet: UnsubackPacket<'_, 1> = decode!(
                UnsubackPacket<1>,
                78,
                [
                    0xB0, 
                    0x4E,

                    0x26, 0x1C,
            
                    0x4A, // Property length

                    // Reason String
                    0x1F, 0x00, 0x0E, b'g', b'e', b't', b' ', b'o', b'u', b't', b't', b'a', b' ', b'h', b'e', b'r', b'e',

                    // User Property
                    0x26, 0x00, 0x07, b'i', b'm', b'a', b'g', b'i', b'n', b'e',
                        0x00, 0x0E, b'a', b'l', b'l', b' ', b't', b'h', b'e', b' ', b'p', b'e', b'o', b'p', b'l', b'e',

                    // User Property
                    0x26, 0x00, 0x05, b'p', b'r', b'i', b'd', b'e',
                        0x00, 0x15, b'(', b'i', b'n', b' ', b't', b'h', b'e', b' ', b'n', b'a', b'm', b'e', b' ', b'o', b'f', b' ', b'l', b'o', b'v', b'e', b')',

                    // Reason codes
                    0x00,
                ]
            );

            assert_eq!(packet.packet_identifier, 9756);
            // assert_eq!(
            //     packet.reason_string,
            //     Some(ReasonString(MqttString::try_from("get outta here").unwrap()))
            // );

            let mut reason_codes: Vec<_, 1> = Vec::new();
            reason_codes.push(ReasonCode::Success).unwrap();
            assert_eq!(packet.reason_codes, reason_codes);
        }
    }
}
