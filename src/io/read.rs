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
    async fn read_and_store(&mut self, len: usize) -> Result<Bytes<'a>, Self::Error>;
}

impl<R: Read, const N: usize> Readable<R> for [u8; N] {
    async fn read(read: &mut R) -> Result<Self, ReadError<<R>::Error>> {
        trace!("reading array of {} bytes", N);

        let mut array = [0; N];
        let mut slice = &mut array[..];
        while !slice.is_empty() {
            match read.read(slice).await? {
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
impl<'b, R: Read, B: BufferProvider<'b>> Read for BodyReader<'_, 'b, R, B>
where
    R::Error: Into<Self::Error>,
{
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
    async fn read_and_store(&mut self, len: usize) -> Result<Bytes<'b>, Self::Error> {
        if self.remaining_len < len {
            return Err(BodyReadError::InsufficientRemainingLen);
        }
        let mut buffer = self
            .buffer
            .provide_buffer(len)
            .map_err(|e: <B as BufferProvider<'b>>::ProvisionError| BodyReadError::Buffer(e))?;

        let slice = buffer.as_mut();

        let mut filled = 0;
        while filled < len {
            match self.read(&mut slice[filled..]).await? {
                0 => return Err(BodyReadError::UnexpectedEOF),
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
    use crate::{io::err::ReadError, io::read::Readable, test::read::SliceReader};

    #[tokio::test]
    #[test_log::test]
    async fn read_u8() {
        let mut r = SliceReader::new(b"\x12");
        let v = u8::read(&mut r).await.unwrap();
        assert_eq!(v, 0x12);
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_u16_u32() {
        let mut r16 = SliceReader::new(b"\x01\x02");
        let v16 = u16::read(&mut r16).await.unwrap();
        assert_eq!(v16, 0x0102);

        let mut r32 = SliceReader::new(b"\x01\x02\x03\x04");
        let v32 = u32::read(&mut r32).await.unwrap();
        assert_eq!(v32, 0x0102_0304);
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_array_and_eof() {
        // exact length
        let mut r = SliceReader::new(b"abc");
        let a: [u8; 3] = <[u8; 3]>::read(&mut r).await.unwrap();
        assert_eq!(&a, b"abc");

        // unexpected EOF when not enough bytes
        let mut r2 = SliceReader::new(b"xy");
        let res = <[u8; 3]>::read(&mut r2).await;
        assert!(matches!(res, Err(ReadError::UnexpectedEOF)));
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_bool_cases() {
        let mut r_false = SliceReader::new(b"\x00");
        let b = bool::read(&mut r_false).await.unwrap();
        assert!(!b);

        let mut r_true = SliceReader::new(b"\x01");
        let b = bool::read(&mut r_true).await.unwrap();
        assert!(b);

        let mut r_bad = SliceReader::new(b"\x02");
        let res = bool::read(&mut r_bad).await;
        assert!(matches!(res, Err(ReadError::MalformedPacket)));
    }
}
