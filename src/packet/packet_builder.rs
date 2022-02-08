use super::mqtt_packet::Packet;
use super::packet_type::PacketType;

// Je potreba vytvori

pub struct PacketBuilder<'a> {
    currentPacket: Packet<'a>,
}

impl<'a> PacketBuilder<'a>  {
    pub fn new(packet: Packet<'a>) -> Self {
        Self{ currentPacket: packet }
    }

    pub fn build(&self) -> &Packet<'a> {
        return &self.currentPacket;
    }

    pub fn decode(&self, buffer: &'a mut [u8]) -> &Packet<'a> {
        return &self.currentPacket;
    }

    pub fn addPacketType(& mut self, packet_type: PacketType) {
        self.currentPacket.header_control = self.currentPacket.header_control & 0x0F;
        self.currentPacket.header_control = self.currentPacket.header_control | <PacketType as Into<u8>>::into(packet_type);
    }

    pub fn addFlags(& mut self, dup: bool, qos: u8, retain: bool) {
        let cur_type: u8 = self.currentPacket.header_control & 0xF0;
        if cur_type != 0x30 {
            log::error!("Cannot add flags into packet with other than PUBLISH type");
            return;
        }
        let mut flags: u8 = 0x00;
        if dup {
            flags = flags | 0x08;
        }
        if qos == 1 {
            flags = flags | 0x02;
        }
        if qos == 2 {
            flags = flags | 0x04;
        }
        if retain {
            flags = flags | 0x01;
        }
        self.currentPacket.header_control = cur_type | flags;
    }

    pub fn completePacket(& mut self) {
        // Tutaj se cely packet dokonci - spocita se remaining len co chybi v hlavicce atd...
    }
}