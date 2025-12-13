use crate::{eio::Read, io::err::ReadError, types::VarByteInt};
use tokio_test::assert_ok;

#[cfg(feature = "alloc")]
use crate::buffer::AllocBuffer;
#[cfg(feature = "bump")]
use crate::buffer::BumpBuffer;

use crate::{
    buffer::BufferProvider,
    header::FixedHeader,
    io::read::{BodyReader, Readable},
    packet::RxPacket,
    test::read::SliceReader,
};

macro_rules! decode {
    ($t:ty, $remaining_len:literal, $bytes:expr) => {{
        const LEN: usize = ($bytes).len();
        const REMAINING_LEN: usize = ($remaining_len);

        let buffer: &mut [u8; LEN] = std::boxed::Box::leak(std::boxed::Box::new([0u8; LEN]));

        crate::test::rx::decode_packet::<$t, _, REMAINING_LEN>(($bytes), buffer).await
    }};
}

impl<R: Read> Readable<R> for FixedHeader {
    async fn read(net: &mut R) -> Result<Self, ReadError<R::Error>> {
        let type_and_flags = u8::read(net).await?;
        let remaining_len = VarByteInt::read(net).await?;
        Ok(Self {
            type_and_flags,
            remaining_len,
        })
    }
}

pub async fn decode_packet<'a, T: RxPacket<'a>, const N: usize, const REMAINING_LEN: usize>(
    bytes: [u8; N],
    buffer: &'a mut [u8],
) -> T {
    let mut reader = SliceReader::new(&bytes);

    let mut buffer = create_buffer(buffer);

    let result = FixedHeader::read(&mut reader).await;
    let header = assert_ok!(result);

    let packet_type = assert_ok!(header.packet_type());
    assert_eq!(packet_type, T::PACKET_TYPE, "Packet type not matching");

    assert_eq!(
        header.remaining_len.size(),
        REMAINING_LEN,
        "Remaining length not matching"
    );

    let reader = BodyReader::new(&mut reader, &mut buffer, REMAINING_LEN);
    let result = T::receive(&header, reader).await;
    let packet = assert_ok!(result);

    packet
}

#[allow(unused_variables)]
fn create_buffer<'a>(buffer: &'a mut [u8]) -> impl BufferProvider<'a> {
    #[cfg(feature = "bump")]
    {
        BumpBuffer::new(buffer)
    }
    #[cfg(feature = "alloc")]
    {
        AllocBuffer
    }

    #[cfg(not(any(feature = "bump", feature = "alloc")))]
    {
        buffer::FailingBuffer
    }
}

#[cfg(not(any(feature = "bump", feature = "alloc")))]
mod buffer {
    use crate::buffer::BufferProvider;

    pub struct FailingBuffer;
    impl<'a> BufferProvider<'a> for FailingBuffer {
        type Buffer = &'a mut [u8];
        type ProvisionError = ();

        fn provide_buffer(&mut self, _: usize) -> Result<Self::Buffer, Self::ProvisionError> {
            Err(())
        }
    }
}

pub(crate) use decode;
