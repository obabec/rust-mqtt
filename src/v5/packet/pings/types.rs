use crate::header::PacketType;

pub struct Req;
pub struct Resp;

pub trait PingPacketType {
    const PACKET_TYPE: PacketType;
    const FLAGS: u8;
}

impl PingPacketType for Req {
    const PACKET_TYPE: PacketType = PacketType::Pingreq;
    const FLAGS: u8 = 0x00;
}
impl PingPacketType for Resp {
    const PACKET_TYPE: PacketType = PacketType::Pingresp;
    const FLAGS: u8 = 0x00;
}
