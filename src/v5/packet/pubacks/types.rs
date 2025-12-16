use crate::{header::PacketType, types::ReasonCode};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ack;
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rec;
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rel;
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Comp;

pub trait PubackPacketType {
    const PACKET_TYPE: PacketType;
    const FLAGS: u8;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool;
}

impl PubackPacketType for Ack {
    const PACKET_TYPE: PacketType = PacketType::Puback;
    const FLAGS: u8 = 0x00;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool {
        is_ack_rec_reason_code(reason_code)
    }
}

impl PubackPacketType for Rec {
    const PACKET_TYPE: PacketType = PacketType::Pubrec;
    const FLAGS: u8 = 0x00;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool {
        is_ack_rec_reason_code(reason_code)
    }
}

impl PubackPacketType for Rel {
    const PACKET_TYPE: PacketType = PacketType::Pubrel;
    const FLAGS: u8 = 0x02;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool {
        is_rel_comp_reason_code(reason_code)
    }
}

impl PubackPacketType for Comp {
    const PACKET_TYPE: PacketType = PacketType::Pubcomp;
    const FLAGS: u8 = 0x00;

    fn reason_code_allowed(reason_code: ReasonCode) -> bool {
        is_rel_comp_reason_code(reason_code)
    }
}

fn is_ack_rec_reason_code(reason_code: ReasonCode) -> bool {
    matches!(
        reason_code,
        ReasonCode::Success
            | ReasonCode::NoMatchingSubscribers
            | ReasonCode::UnspecifiedError
            | ReasonCode::ImplementationSpecificError
            | ReasonCode::NotAuthorized
            | ReasonCode::TopicNameInvalid
            | ReasonCode::PacketIdentifierInUse
            | ReasonCode::QuotaExceeded
            | ReasonCode::PayloadFormatInvalid
    )
}

fn is_rel_comp_reason_code(reason_code: ReasonCode) -> bool {
    matches!(
        reason_code,
        ReasonCode::Success | ReasonCode::PacketIdentifierNotFound
    )
}
