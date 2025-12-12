use core::marker::PhantomData;

use crate::{
    buffer::BufferProvider,
    eio::{Read, Write},
    fmt::{error, trace},
    header::{FixedHeader, PacketType},
    io::{read::BodyReader, write::Writable},
    packet::{Packet, RxError, RxPacket, TxError, TxPacket},
    types::VarByteInt,
    v5::packet::pings::types::{PingPacketType, Req, Resp},
};

mod types;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericPingPacket<T: PingPacketType> {
    phantom_data: PhantomData<T>,
}

pub type PingreqPacket = GenericPingPacket<Req>;
pub type PingrespPacket = GenericPingPacket<Resp>;

impl<T: PingPacketType> Packet for GenericPingPacket<T> {
    const PACKET_TYPE: PacketType = T::PACKET_TYPE;
}
impl<'p, T: PingPacketType> RxPacket<'p> for GenericPingPacket<T> {
    async fn receive<R: Read, B: BufferProvider<'p>>(
        header: &FixedHeader,
        _: BodyReader<'_, 'p, R, B>,
    ) -> Result<Self, RxError<R::Error, B::ProvisionError>> {
        trace!("decoding");

        if header.flags() != T::FLAGS {
            error!("flags are not 0");
            Err(RxError::MalformedPacket)
        } else {
            Ok(Self {
                phantom_data: PhantomData,
            })
        }
    }
}
impl<T: PingPacketType> TxPacket for GenericPingPacket<T> {
    async fn send<W: Write>(&self, write: &mut W) -> Result<(), TxError<W::Error>> {
        // Safety: 0 < VarByteInt::MAX_ENCODABLE
        let remaining_len = unsafe { VarByteInt::new_unchecked(0) };

        FixedHeader::new(Self::PACKET_TYPE, T::FLAGS, remaining_len)
            .write(write)
            .await?;

        Ok(())
    }
}

impl<T: PingPacketType> GenericPingPacket<T> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

#[cfg(test)]
mod unit {
    mod req {
        use crate::{
            test::{rx::decode, tx::encode},
            v5::packet::PingreqPacket,
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode() {
            #[rustfmt::skip]
            encode!(
                PingreqPacket::new(),
                [
                    0xC0, //
                    0x00, // remaining length
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode() {
            decode!(PingreqPacket, 0, [0xC0, 0x00]);
        }
    }

    mod resp {
        use crate::{
            test::{rx::decode, tx::encode},
            v5::packet::PingrespPacket,
        };

        #[tokio::test]
        #[test_log::test]
        async fn encode() {
            #[rustfmt::skip]
            encode!(
                PingrespPacket::new(),
                [
                    0xD0, //
                    0x00, // remaining length
                ]
            );
        }

        #[tokio::test]
        #[test_log::test]
        async fn decode() {
            decode!(PingrespPacket, 0, [0xD0, 0x00]);
        }
    }
}
