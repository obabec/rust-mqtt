use super::mqtt_packet::Packet;
use super::packet_type::PacketType;

pub struct PacketBuilder<'a> {
    currentPacket: Packet<'a>,
}

impl<'a> PacketBuilder<'a>  {
    pub fn build(&self) -> &Packet<'a> {
        return &self.currentPacket;
    }

    pub fn decode(&self, buffer: &'a mut [u8]) -> &Packet<'a> {
        return &self.currentPacket;
    }

    pub fn addPacketType(packet_type: PacketType) {

    }
}