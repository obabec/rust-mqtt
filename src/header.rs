use crate::{
    eio::Write,
    fmt::unreachable,
    io::{
        err::WriteError,
        write::{Writable, wlen},
    },
    types::VarByteInt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FixedHeader {
    pub(crate) type_and_flags: u8,
    pub(crate) remaining_len: VarByteInt,
}

impl Writable for FixedHeader {
    fn written_len(&self) -> usize {
        wlen!(u8) + self.remaining_len.written_len()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.type_and_flags.write(write).await?;
        self.remaining_len.write(write).await?;

        Ok(())
    }
}

impl FixedHeader {
    pub(crate) fn new(packet_type: PacketType, flags: u8, remaining_len: VarByteInt) -> Self {
        let packet_type = (packet_type as u8) << 4;
        Self {
            type_and_flags: packet_type | flags,
            remaining_len,
        }
    }

    pub fn flags(&self) -> u8 {
        self.type_and_flags & 0x0F
    }

    pub fn packet_type(&self) -> Result<PacketType, Reserved> {
        PacketType::from_type_and_flags(self.type_and_flags)
    }
}

/// Returned if packet type is reserved
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Reserved;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PacketType {
    Connect = 1,
    Connack = 2,
    Publish = 3,
    Puback = 4,
    Pubrec = 5,
    Pubrel = 6,
    Pubcomp = 7,
    Subscribe = 8,
    Suback = 9,
    Unsubscribe = 10,
    Unsuback = 11,
    Pingreq = 12,
    Pingresp = 13,
    Disconnect = 14,

    #[cfg(feature = "v5")]
    Auth = 15,
}

impl PacketType {
    pub fn from_type_and_flags(type_and_flags: u8) -> Result<Self, Reserved> {
        match type_and_flags >> 4 {
            0 => Err(Reserved),
            1 => Ok(PacketType::Connect),
            2 => Ok(PacketType::Connack),
            3 => Ok(PacketType::Publish),
            4 => Ok(PacketType::Puback),
            5 => Ok(PacketType::Pubrec),
            6 => Ok(PacketType::Pubrel),
            7 => Ok(PacketType::Pubcomp),
            8 => Ok(PacketType::Subscribe),
            9 => Ok(PacketType::Suback),
            10 => Ok(PacketType::Unsubscribe),
            11 => Ok(PacketType::Unsuback),
            12 => Ok(PacketType::Pingreq),
            13 => Ok(PacketType::Pingresp),
            14 => Ok(PacketType::Disconnect),

            #[cfg(feature = "v3")]
            15 => Err(PacketTypeError::Reserved),

            #[cfg(feature = "v5")]
            15 => Ok(PacketType::Auth),

            _ => unreachable!(),
        }
    }
}
