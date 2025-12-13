use core::{cmp::min, marker::PhantomData};

use crate::{
    buffer::BufferProvider,
    bytes::Bytes,
    eio::{ErrorType, Read},
    fmt::trace,
    io::err::{BodyReadError, ReadError},
    types::{MqttBinary, MqttString},
};

pub trait Readable<R: Read>: Sized {
    async fn read(read: &mut R) -> Result<Self, ReadError<R::Error>>;
}

pub trait Store<'a>: Read {
    async fn read_and_store(&mut self, len: usize) -> Result<Bytes<'a>, ReadError<Self::Error>>;
}

impl<R: Read, const N: usize> Readable<R> for [u8; N] {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        trace!("reading array of {} bytes", N);

        let mut array = [0; N];
        let mut slice = &mut array[..];
        while !slice.is_empty() {
            match read.read(slice).await.map_err(ReadError::Read)? {
                0 => return Err(ReadError::UnexpectedEOF),
                n => slice = &mut slice[n..],
            }
        }
        Ok(array)
    }
}
impl<R: Read> Readable<R> for u8 {
    async fn read(read: &mut R) -> Result<Self, ReadError<R::Error>> {
        <[u8; 1]>::read(read).await.map(Self::from_be_bytes)
    }
}
impl<R: Read> Readable<R> for u16 {
    async fn read(read: &mut R) -> Result<Self, ReadError<R::Error>> {
        <[u8; 2]>::read(read).await.map(Self::from_be_bytes)
    }
}
impl<R: Read> Readable<R> for u32 {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        <[u8; 4]>::read(read).await.map(Self::from_be_bytes)
    }
}
impl<R: Read> Readable<R> for bool {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        match u8::read(read).await? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ReadError::MalformedPacket),
        }
    }
}
impl<'b, R: Read + Store<'b>> Readable<R> for MqttBinary<'b> {
    async fn read(read: &mut R) -> Result<Self, ReadError<R::Error>> {
        let len = u16::read(read).await? as usize;

        trace!("reading slice of {} bytes", len);

        Ok(MqttBinary(read.read_and_store(len).await?))
    }
}
impl<'s, R: Read + Store<'s>> Readable<R> for MqttString<'s> {
    async fn read(read: &mut R) -> Result<Self, ReadError<R::Error>> {
        MqttBinary::read(read)
            .await?
            .try_into()
            .map_err(|_| ReadError::MalformedPacket)
    }
}

pub struct BodyReader<'r, 'b, R: Read, B: BufferProvider<'b>> {
    r: &'r mut R,
    buffer: &'r mut B,
    remaining_len: usize,
    _b: PhantomData<&'b ()>,
}

impl<'b, R: Read, B: BufferProvider<'b>> ErrorType for BodyReader<'_, 'b, R, B> {
    type Error = BodyReadError<R::Error, B::ProvisionError>;
}
impl<'b, R: Read, B: BufferProvider<'b>> Read for BodyReader<'_, 'b, R, B> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if !buf.is_empty() && self.remaining_len == 0 {
            return Err(BodyReadError::InsufficientRemainingLen);
        }
        let len = min(buf.len(), self.remaining_len);
        let buf = &mut buf[..len];
        let read = self.r.read(buf).await?;
        self.remaining_len -= read;
        Ok(read)
    }
}
impl<'r, 'b, R: Read, B: BufferProvider<'b>> Store<'b> for BodyReader<'r, 'b, R, B> {
    async fn read_and_store(&mut self, len: usize) -> Result<Bytes<'b>, ReadError<Self::Error>> {
        if self.remaining_len < len {
            return Err(ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }
        let mut buffer = self
            .buffer
            .provide_buffer(len)
            .map_err(BodyReadError::Buffer)?;

        let slice = buffer.as_mut();

        let mut filled = 0;
        while filled < len {
            match self.read(&mut slice[filled..]).await? {
                0 => return Err(ReadError::UnexpectedEOF),
                n => filled += n,
            }
        }

        Ok(buffer.into())
    }
}

impl<'r, 'b, R: Read, B: BufferProvider<'b>> BodyReader<'r, 'b, R, B> {
    pub fn new(r: &'r mut R, buffer: &'r mut B, remaining_len: usize) -> Self {
        Self {
            r,
            buffer,
            remaining_len,
            _b: PhantomData,
        }
    }

    pub fn remaining_len(&self) -> usize {
        self.remaining_len
    }

    pub async fn skip(
        &mut self,
        len: usize,
    ) -> Result<(), BodyReadError<R::Error, B::ProvisionError>> {
        self.remaining_len -= len;
        let mut missing = len;

        const CHUNK_SIZE: usize = 16;
        let mut buf = [0; CHUNK_SIZE];
        while missing > 0 {
            let buf = &mut buf[0..min(CHUNK_SIZE, missing)];
            match self.r.read(buf).await? {
                0 => return Err(BodyReadError::UnexpectedEOF),
                r => missing -= r,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod unit {
    mod readable {
        use tokio_test::{assert_err, assert_ok};

        use crate::{io::err::ReadError, io::read::Readable, test::read::SliceReader};

        #[tokio::test]
        #[test_log::test]
        async fn read_array() {
            let mut r = SliceReader::new(b"abcdefghijklmnopqrstuvwxyz");
            let a = assert_ok!(<[u8; 26]>::read(&mut r).await);
            assert_eq!(&a, b"abcdefghijklmnopqrstuvwxyz");
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_u8() {
            let mut r = SliceReader::new(b"\x12");
            let v = assert_ok!(u8::read(&mut r).await);
            assert_eq!(v, 0x12);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_u16() {
            let mut r = SliceReader::new(b"\x01\x02");
            let v = assert_ok!(u16::read(&mut r).await);
            assert_eq!(v, 0x0102);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_u32() {
            let mut r = SliceReader::new(b"\x01\x02\x03\x04");
            let v = assert_ok!(u32::read(&mut r).await);
            assert_eq!(v, 0x01020304);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_bool() {
            let mut r_false = SliceReader::new(b"\x00");
            let b = assert_ok!(bool::read(&mut r_false).await);
            assert!(!b);

            let mut r_true = SliceReader::new(b"\x01");
            let b = assert_ok!(bool::read(&mut r_true).await);
            assert!(b);

            let mut r_bad = SliceReader::new(b"\x02");
            let res = bool::read(&mut r_bad).await;
            assert!(matches!(res, Err(ReadError::MalformedPacket)));
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_eof() {
            let mut r = SliceReader::new(b"abcdefghijklmno");
            let e = assert_err!(<[u8; 16]>::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            let mut r = SliceReader::new(b"");
            let e = assert_err!(u8::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            let mut r = SliceReader::new(b"\x00");
            let e = assert_err!(u16::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            let mut r = SliceReader::new(b"\x00\x00\x00");
            let e = assert_err!(u32::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }
    }

    mod body_reader {
        use tokio_test::{assert_err, assert_ok};

        #[cfg(feature = "alloc")]
        use crate::buffer::AllocBuffer;
        #[cfg(feature = "bump")]
        use crate::buffer::BumpBuffer;

        use crate::{
            io::{
                err::{BodyReadError, ReadError},
                read::{BodyReader, Readable},
            },
            test::read::SliceReader,
            types::{MqttBinary, MqttString},
        };

        #[tokio::test]
        #[test_log::test]
        async fn read_array() {
            let mut s = SliceReader::new(b"abcdefghijklmnopqrstuvwxyz");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 26);
            let a = assert_ok!(<[u8; 26]>::read(&mut r).await);
            assert_eq!(&a, b"abcdefghijklmnopqrstuvwxyz");
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_u8() {
            let mut s = SliceReader::new(b"\x12");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 1);
            let v = assert_ok!(u8::read(&mut r).await);
            assert_eq!(v, 0x12);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_u16() {
            let mut s = SliceReader::new(b"\x01\x02");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 2);
            let v = assert_ok!(u16::read(&mut r).await);
            assert_eq!(v, 0x0102);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_u32() {
            let mut s = SliceReader::new(b"\x01\x02\x03\x04");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 4);
            let v = assert_ok!(u32::read(&mut r).await);
            assert_eq!(v, 0x01020304);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_bool() {
            let mut s_false = SliceReader::new(b"\x00");
            #[cfg(feature = "alloc")]
            let mut b_false = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b_false = [0; 64];
            #[cfg(feature = "bump")]
            let mut b_false = BumpBuffer::new(&mut b_false);

            let mut r_false = BodyReader::new(&mut s_false, &mut b_false, 1);
            let v = assert_ok!(bool::read(&mut r_false).await);
            assert!(!v);

            let mut s_true = SliceReader::new(b"\x01");
            #[cfg(feature = "alloc")]
            let mut b_true = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b_true = [0; 64];
            #[cfg(feature = "bump")]
            let mut b_true = BumpBuffer::new(&mut b_true);

            let mut r_true = BodyReader::new(&mut s_true, &mut b_true, 1);
            let v = assert_ok!(bool::read(&mut r_true).await);
            assert!(v);

            let mut s_bad = SliceReader::new(b"\x02");
            #[cfg(feature = "alloc")]
            let mut b_bad = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b_bad = [0; 64];
            #[cfg(feature = "bump")]
            let mut b_bad = BumpBuffer::new(&mut b_bad);

            let mut r_bad = BodyReader::new(&mut s_bad, &mut b_bad, 1);
            let res = bool::read(&mut r_bad).await;
            assert!(matches!(res, Err(ReadError::MalformedPacket)));
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_binary() {
            let mut s = SliceReader::new(&[0x00, 0x05, 0x01, 0x02, 0x03, 0x04, 0xFF]);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 7);
            let v = assert_ok!(MqttBinary::read(&mut r).await);
            assert_eq!(v.as_ref(), &[0x01, 0x02, 0x03, 0x04, 0xFF]);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_string() {
            let mut s = SliceReader::new(&[
                0x00, 0x09, b'r', b'u', b's', b't', b'-', b'm', b'q', b't', b't',
            ]);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 11);
            let v = assert_ok!(MqttString::read(&mut r).await);
            assert_eq!(v.as_ref(), "rust-mqtt");
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_stream() {
            #[rustfmt::skip]
            let mut s = SliceReader::new(
                &[
                    0x42,                                    // u8
                    0x01, 0x02,                              // u16
                    0x01,                                    // bool (true)
                    0xDE, 0xAD, 0xBE, 0xEF,                  // u32
                    0x00, 0x03, 0xAA, 0xBB, 0xCC,            // binary
                    0x00, 0x04, b't', b'e', b's', b't',      // string
                    0x11, 0x22, 0x33,                        // array[3]
                ]
            );
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 22);

            let v_u8 = assert_ok!(u8::read(&mut r).await);
            assert_eq!(v_u8, 0x42);

            let v_u16 = assert_ok!(u16::read(&mut r).await);
            assert_eq!(v_u16, 0x0102);

            let v_bool = assert_ok!(bool::read(&mut r).await);
            assert!(v_bool);

            let v_u32 = assert_ok!(u32::read(&mut r).await);
            assert_eq!(v_u32, 0xDEADBEEF);

            let v_binary = assert_ok!(MqttBinary::read(&mut r).await);
            assert_eq!(v_binary.as_ref(), &[0xAA, 0xBB, 0xCC]);

            let v_string = assert_ok!(MqttString::read(&mut r).await);
            assert_eq!(v_string.as_ref(), "test");

            let v_array = assert_ok!(<[u8; 3]>::read(&mut r).await);
            assert_eq!(v_array, [0x11, 0x22, 0x33]);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_eof() {
            let mut s = SliceReader::new(b"abcdefghijklmno");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 16);
            let e = assert_err!(<[u8; 16]>::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            let mut s = SliceReader::new(b"");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 1);
            let e = assert_err!(u8::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            let mut s = SliceReader::new(b"\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 2);
            let e = assert_err!(u16::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            let mut s = SliceReader::new(b"\x00\x00\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 4);
            let e = assert_err!(u32::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            // MqttBinary - EOF when reading length
            let mut s = SliceReader::new(b"\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 2);
            let e = assert_err!(MqttBinary::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            // MqttBinary - EOF when reading data
            let mut s = SliceReader::new(&[0x00, 0x05, 0x01, 0x02]);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 7);
            let e = assert_err!(MqttBinary::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            // MqttString - EOF when reading length
            let mut s = SliceReader::new(b"\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 2);
            let e = assert_err!(MqttString::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);

            // MqttString - EOF when reading data
            let mut s = SliceReader::new(&[0x00, 0x04, b't', b'e']);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 6);
            let e = assert_err!(MqttString::read(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_insufficient_remaining_len_array() {
            let mut s = SliceReader::new(b"abcdefghijklmno");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 15);
            let e = assert_err!(<[u8; 16]>::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            let mut s = SliceReader::new(b"abcdefghijklmnop");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 15);
            let e = assert_err!(<[u8; 16]>::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }
        #[tokio::test]
        #[test_log::test]
        async fn read_insufficient_remaining_len_u8() {
            let mut s = SliceReader::new(b"");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 0);
            let e = assert_err!(u8::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            let mut s = SliceReader::new(b"");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 0);
            let e = assert_err!(u8::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }
        #[tokio::test]
        #[test_log::test]
        async fn read_insufficient_remaining_len_u16() {
            let mut s = SliceReader::new(b"\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 1);
            let e = assert_err!(u16::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            let mut s = SliceReader::new(b"\x00\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 1);
            let e = assert_err!(u16::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }
        #[tokio::test]
        #[test_log::test]
        async fn read_insufficient_remaining_len_u32() {
            let mut s = SliceReader::new(b"\x00\x00\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 3);
            let e = assert_err!(u32::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            let mut s = SliceReader::new(b"\x00\x00\x00\x00");
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 3);
            let e = assert_err!(u32::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_insufficient_remaining_len_binary() {
            // Insufficient remaining length when reading length prefix
            let mut s = SliceReader::new(&[0x00, 0x05, 0x01, 0x02, 0x03, 0x04, 0xFF]);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 1);
            let e = assert_err!(MqttBinary::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            // Insufficient remaining length when reading data
            let mut s = SliceReader::new(&[0x00, 0x05, 0x01, 0x02, 0x03, 0x04, 0xFF]);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 6);
            let e = assert_err!(MqttBinary::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            // Insufficient remaining length with exact length prefix bytes but not enough for data
            let mut s = SliceReader::new(&[0x00, 0x05, 0x01, 0x02, 0x03, 0x04, 0xFF]);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 2);
            let e = assert_err!(MqttBinary::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }

        #[tokio::test]
        #[test_log::test]
        async fn read_insufficient_remaining_len_string() {
            // Insufficient remaining length when reading length prefix
            let mut s = SliceReader::new(&[0x00, 0x04, b't', b'e', b's', b't']);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 1);
            let e = assert_err!(MqttString::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            // Insufficient remaining length when reading data
            let mut s = SliceReader::new(&[0x00, 0x04, b't', b'e', b's', b't']);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 5);
            let e = assert_err!(MqttString::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));

            // Insufficient remaining length with exact length prefix bytes but not enough for data
            let mut s = SliceReader::new(&[0x00, 0x04, b't', b'e', b's', b't']);
            #[cfg(feature = "alloc")]
            let mut b = AllocBuffer;
            #[cfg(feature = "bump")]
            let mut b = [0; 64];
            #[cfg(feature = "bump")]
            let mut b = BumpBuffer::new(&mut b);

            let mut r = BodyReader::new(&mut s, &mut b, 2);
            let e = assert_err!(MqttString::read(&mut r).await);
            assert_eq!(e, ReadError::Read(BodyReadError::InsufficientRemainingLen));
        }
    }
}
