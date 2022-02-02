// x x x x - - - -


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
        match orig {
            0x00 =>  return PacketType::Reserved,
            0x10 => return PacketType::Connect,
            0x20 => return PacketType::Connack,
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

