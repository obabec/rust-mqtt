mod rx;
mod tx;

pub use rx::*;
pub use tx::*;

use crate::header::PacketType;

pub trait Packet {
    const PACKET_TYPE: PacketType;
}
