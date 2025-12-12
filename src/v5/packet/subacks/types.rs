use crate::{header::PacketType, types::ReasonCode};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Suback;
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Unsuback;

pub trait SubackPacketType {
    const PACKET_TYPE: PacketType;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool;
}

impl SubackPacketType for Suback {
    const PACKET_TYPE: PacketType = PacketType::Suback;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool {
        matches!(
            reason_code,
            ReasonCode::Success
                | ReasonCode::GrantedQoS1
                | ReasonCode::GrantedQoS2
                | ReasonCode::UnspecifiedError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::NotAuthorized
                | ReasonCode::TopicFilterInvalid
                | ReasonCode::PacketIdentifierInUse
                | ReasonCode::QuotaExceeded
                | ReasonCode::SharedSubscriptionsNotSupported
                | ReasonCode::SubscriptionIdentifiersNotSupported
                | ReasonCode::WildcardSubscriptionsNotSupported
        )
    }
}

impl SubackPacketType for Unsuback {
    const PACKET_TYPE: PacketType = PacketType::Unsuback;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool {
        matches!(
            reason_code,
            ReasonCode::Success
                | ReasonCode::NoSubscriptionExisted
                | ReasonCode::UnspecifiedError
                | ReasonCode::ImplementationSpecificError
                | ReasonCode::NotAuthorized
                | ReasonCode::TopicFilterInvalid
                | ReasonCode::PacketIdentifierInUse
        )
    }
}
