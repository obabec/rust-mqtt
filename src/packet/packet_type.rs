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
    Auth
}

impl From<u8> for PacketType {
    fn from(orig: u8) -> Self {
        let packet_type: u8 = orig & 0xF0;
        match packet_type {
            0x10 => return PacketType::Connect,
            0x20 => return PacketType::Connack,
            0x00 => return PacketType::Reserved,
            0x30 => return PacketType::Publish,
            0x40 => return PacketType::Puback,
            0x50 => return PacketType::Pubrec,
            0x60 => return PacketType::Pubrel,
            0x70 => return PacketType::Pubcomp,
            0x80 => return PacketType::Subscribe,
            0x90 => return PacketType::Suback,
            0xA0 => return PacketType::Unsubscribe,
            0xB0 => return PacketType::Unsuback,
            0xC0 => return PacketType::Pingreq,
            0xD0 => return PacketType::Pingresp,
            0xE0 => return PacketType::Disconnect,
            0xF0 => return PacketType::Auth,
            _ => return PacketType::Reserved
        };
    }
}

impl Into<u8> for PacketType {
    fn into(self) -> u8 {
        match self {
            PacketType::Connect => return 0x00,
            PacketType::Connack => return 0x10,
            PacketType::Reserved => return 0x20,
            PacketType::Publish => return 0x30,
            PacketType::Puback => return 0x40,
            PacketType::Pubrec => return 0x50,
            PacketType::Pubrel => return 0x60,
            PacketType::Pubcomp => return 0x70,
            PacketType::Subscribe => return 0x80,
            PacketType::Suback => return 0x90,
            PacketType::Unsubscribe => return 0xA0,
            PacketType::Unsuback => return 0xB0,
            PacketType::Pingreq => return 0xC0,
            PacketType::Pingresp => return 0xD0,
            PacketType::Disconnect => return 0xE0,
            PacketType::Auth => return 0xF0,
            PacketType::Reserved => return 0x00
        }
    }
}

