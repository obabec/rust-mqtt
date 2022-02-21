// x x x x - - - -

#[derive(PartialEq)]
pub enum PacketType {
    Reserved,
    Connect,
    Connack,
    Publish,
    Puback,
    Pubrec,
    Pubrel,
    Pubcomp,
    Subscribe,
    Suback,
    Unsubscribe,
    Unsuback,
    Pingreq,
    Pingresp,
    Disconnect,
    Auth,
}

impl From<u8> for PacketType {
    fn from(orig: u8) -> Self {
        let packet_type: u8 = orig & 0xF0;
        return match packet_type {
            0x10 => PacketType::Connect,
            0x20 => PacketType::Connack,
            0x00 => PacketType::Reserved,
            0x30 => PacketType::Publish,
            0x40 => PacketType::Puback,
            0x50 => PacketType::Pubrec,
            0x60 => PacketType::Pubrel,
            0x70 => PacketType::Pubcomp,
            0x80 => PacketType::Subscribe,
            0x90 => PacketType::Suback,
            0xA0 => PacketType::Unsubscribe,
            0xB0 => PacketType::Unsuback,
            0xC0 => PacketType::Pingreq,
            0xD0 => PacketType::Pingresp,
            0xE0 => PacketType::Disconnect,
            0xF0 => PacketType::Auth,
            _ => PacketType::Reserved,
        };
    }
}

impl Into<u8> for PacketType {
    fn into(self) -> u8 {
        return match self {
            PacketType::Connect => 0x10,
            PacketType::Connack => 0x20,
            PacketType::Publish => 0x30,
            PacketType::Puback => 0x40,
            PacketType::Pubrec => 0x50,
            PacketType::Pubrel => 0x60,
            PacketType::Pubcomp => 0x70,
            PacketType::Subscribe => 0x82,
            PacketType::Suback => 0x90,
            PacketType::Unsubscribe => 0xA0,
            PacketType::Unsuback => 0xB0,
            PacketType::Pingreq => 0xC0,
            PacketType::Pingresp => 0xD0,
            PacketType::Disconnect => 0xE0,
            PacketType::Auth => 0xF0,
            PacketType::Reserved => 0x00,
        };
    }
}
