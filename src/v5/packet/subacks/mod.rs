use core::marker::PhantomData;

use heapless::Vec;

use crate::{
    buffer::BufferProvider,
    eio::Read,
    fmt::{trace, verbose},
    header::{FixedHeader, PacketType},
    io::{
        read::{BodyReader, Readable},
        write::{Writable, wlen},
    },
    packet::{Packet, RxError, RxPacket},
    types::{PacketIdentifier, ReasonCode, VarByteInt},
    v5::{
        packet::subacks::types::{Suback, SubackPacketType, Unsuback},
        property::{PropertyType, UserProperty},
    },
};

mod types;

pub type SubackPacket<'p, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> =
    GenericSubackPacket<'p, Suback, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>;
pub type UnsubackPacket<'p, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> =
    GenericSubackPacket<'p, Unsuback, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericSubackPacket<
    'p,
    T,
    const MAX_TOPIC_FILTERS: usize,
    const MAX_USER_PROPERTIES: usize,
> {
    pub packet_identifier: PacketIdentifier,

    // reason string is currently unused and does not have to be read into memory.
    // reason_string: Option<ReasonString<'p>>,
    pub user_properties: Vec<UserProperty<'p>, MAX_USER_PROPERTIES>,

    pub reason_codes: Vec<ReasonCode, MAX_TOPIC_FILTERS>,
    _phantom_data: PhantomData<&'p T>,
}

impl<T: SubackPacketType, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize> Packet
    for GenericSubackPacket<'_, T, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    const PACKET_TYPE: PacketType = T::PACKET_TYPE;
}
impl<'p, T: SubackPacketType, const MAX_TOPIC_FILTERS: usize, const MAX_USER_PROPERTIES: usize>
    RxPacket<'p> for GenericSubackPacket<'p, T, MAX_TOPIC_FILTERS, MAX_USER_PROPERTIES>
{
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        mut reader: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding {:?} packet", T::PACKET_TYPE);

        if header.flags() != 0 {
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

        verbose!("reading property length field");
        let mut properties_length = VarByteInt::read(r).await?.size();

        verbose!("property length: {} bytes", properties_length);

        if properties_length > r.remaining_len() {
            trace!(
                "invalid {:?} property length for remaining packet length",
                T::PACKET_TYPE
            );
            return Err(RxError::MalformedPacket);
        }
        if properties_length == r.remaining_len() {
            trace!("{:?} packet does not contain a reason code", T::PACKET_TYPE);
            return Err(RxError::ProtocolError);
        }

        let mut seen_reason_string = false;
        let mut user_properties = Vec::new();

        while properties_length > 0 {
            verbose!(
                "reading property identifier (remaining length: {} bytes)",
                r.remaining_len()
            );
            let property_type = PropertyType::read(r).await?;

            // unchecked sub because `properties_length` > 0
            properties_length -= property_type.written_len();

            verbose!(
                "reading {:?} property body (remaining length: {} bytes)",
                property_type,
                r.remaining_len()
            );

            #[rustfmt::skip]
            match property_type {
                PropertyType::ReasonString if seen_reason_string => return Err(RxError::ProtocolError),
                PropertyType::ReasonString => {
                    seen_reason_string = true;
                    let len = u16::read(r).await? as usize;
                    verbose!("skipping reason string ({} bytes)", len);
                    r.skip(len).await?;
                    properties_length = properties_length
                        .checked_sub(wlen!(u16) + len)
                        .ok_or(RxError::MalformedPacket)?;
                },
                PropertyType::UserProperty if !user_properties.is_full() => {
                    let user_property = UserProperty::read(r).await?;

                    properties_length = properties_length
                        .checked_sub(user_property.0.written_len())
                        .ok_or(RxError::MalformedPacket)?;

                    // Safety: `!Vec::is_full` guarantees there is space
                    unsafe { user_properties.push_unchecked(user_property) };
                }
                PropertyType::UserProperty => {
                    let len = UserProperty::skip(r).await?;
                    properties_length = properties_length
                        .checked_sub(len)
                        .ok_or(RxError::MalformedPacket)?;
                }
                p => {
                    // Malformed packet according to <https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html#_Toc3901029>
                    trace!("invalid {:?} property: {:?}", T::PACKET_TYPE, p);
                    return Err(RxError::MalformedPacket)
                },
            };

            let _ = seen_reason_string;
        }

        let mut reason_codes = Vec::new();
        while r.remaining_len() > 0 {
            verbose!("reading reason code field");
            let reason_code = ReasonCode::read(r).await?;

            if !T::reason_code_allowed(reason_code) {
                trace!(
                    "invalid {:?} reason code: {:?}",
                    T::PACKET_TYPE,
                    reason_code
                );
                return Err(RxError::ProtocolError);
            }

            // This currently doesn't return a const space related error as we only send SUBSCRIBE packets with a single
            // reason code and therefore expect the server to send ACKs with one reason code as well.
            // This is not really the correct error in the decoding sense but it's ok and more efficient for now and allows
            // us to get rid of the public error counterpart.
            reason_codes
                .push(reason_code)
                .map_err(|_| RxError::ProtocolError)?;
        }

        Ok(Self {
            packet_identifier,
            user_properties,
            reason_codes,
            _phantom_data: PhantomData,
        })
    }
}

#[cfg(test)]
mod unit {
    mod suback {
        use core::num::NonZero;

        use heapless::Vec;

        use crate::{
            test::rx::decode,
            types::{MqttString, MqttStringPair, PacketIdentifier, ReasonCode},
            v5::{packet::SubackPacket, property::UserProperty},
        };

        #[tokio::test]
        #[test_log::test]
        async fn decode_payload() {
            #[rustfmt::skip]
            let packet = decode!(
                SubackPacket<12, 16>,
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

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(6025).unwrap())
            );
            // assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());

            let reason_codes: Vec<_, 12> = [
                ReasonCode::Success,
                ReasonCode::WildcardSubscriptionsNotSupported,
                ReasonCode::GrantedQoS1,
                ReasonCode::SubscriptionIdentifiersNotSupported,
                ReasonCode::GrantedQoS2,
                ReasonCode::SharedSubscriptionsNotSupported,
                ReasonCode::UnspecifiedError,
                ReasonCode::QuotaExceeded,
                ReasonCode::ImplementationSpecificError,
                ReasonCode::PacketIdentifierInUse,
                ReasonCode::NotAuthorized,
                ReasonCode::TopicFilterInvalid,
            ]
            .into();
            assert_eq!(packet.reason_codes, reason_codes);
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(
                SubackPacket<1, 16>,
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

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(5620).unwrap())
            );
            // assert_eq!(
            //     packet.reason_string,
            //     Some(ReasonString(MqttString::try_from("crazy things").unwrap()))
            // );
            assert_eq!(
                packet.user_properties,
                [
                    UserProperty(MqttStringPair::new(
                        MqttString::from_str("some name").unwrap(),
                        MqttString::from_str("any value").unwrap()
                    )),
                    UserProperty(MqttStringPair::new(
                        MqttString::from_str("any key").unwrap(),
                        MqttString::from_str("a value").unwrap()
                    ))
                ]
            );

            let reason_codes: Vec<_, 1> = [ReasonCode::Success].into();
            assert_eq!(packet.reason_codes, reason_codes);
        }
    }

    mod unsuback {
        use core::num::NonZero;

        use heapless::Vec;

        use crate::{
            test::rx::decode,
            types::{MqttString, MqttStringPair, PacketIdentifier, ReasonCode},
            v5::{packet::UnsubackPacket, property::UserProperty},
        };

        #[tokio::test]
        #[test_log::test]
        async fn decode_payload() {
            #[rustfmt::skip]
            let packet = decode!(
                UnsubackPacket<7, 16>,
                10,
                [
                    0xB0,
                    0x0A,

                    0xA3, 0xF4, 0x00, 
                    
                    // Reason codes
                    0x00, 0x91, 0x11, 0x8F, 0x80, 0x87, 0x83,
                ]
            );

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(41972).unwrap())
            );
            // assert!(packet.reason_string.is_none());
            assert!(packet.user_properties.is_empty());

            let reason_codes: Vec<_, 7> = [
                ReasonCode::Success,
                ReasonCode::PacketIdentifierInUse,
                ReasonCode::NoSubscriptionExisted,
                ReasonCode::TopicFilterInvalid,
                ReasonCode::UnspecifiedError,
                ReasonCode::NotAuthorized,
                ReasonCode::ImplementationSpecificError,
            ]
            .into();

            assert_eq!(packet.reason_codes, reason_codes);
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode_properties() {
            #[rustfmt::skip]
            let packet = decode!(
                UnsubackPacket<1, 16>,
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

            assert_eq!(
                packet.packet_identifier,
                PacketIdentifier::new(NonZero::new(9756).unwrap())
            );
            // assert_eq!(
            //     packet.reason_string,
            //     Some(ReasonString(MqttString::try_from("get outta here").unwrap()))
            // );
            assert_eq!(
                packet.user_properties,
                [
                    UserProperty(MqttStringPair::new(
                        MqttString::from_str("imagine").unwrap(),
                        MqttString::from_str("all the people").unwrap()
                    )),
                    UserProperty(MqttStringPair::new(
                        MqttString::from_str("pride").unwrap(),
                        MqttString::from_str("(in the name of love)").unwrap()
                    ))
                ]
            );

            let reason_codes: Vec<_, 1> = [ReasonCode::Success].into();
            assert_eq!(packet.reason_codes, reason_codes);
        }
    }
}
