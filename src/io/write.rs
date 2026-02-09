use crate::{
    eio::Write,
    io::err::WriteError,
    types::{MqttBinary, MqttString, TopicName, VarByteInt},
};

pub trait Writable {
    fn written_len(&self) -> usize;
    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>>;
}

macro_rules! wlen {
    ( $W:ty ) => {
        <$W>::default().written_len()
    };
}

impl<T: Writable> Writable for Option<T> {
    fn written_len(&self) -> usize {
        self.as_ref().map(|t| t.written_len()).unwrap_or_default()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        match self {
            Some(t) => t.write(write).await,
            None => Ok(()),
        }
    }
}

impl Writable for [u8] {
    fn written_len(&self) -> usize {
        self.len()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let mut slice = self;
        while !slice.is_empty() {
            match write.write(slice).await? {
                0 => return Err(WriteError::WriteZero),
                n => slice = &slice[n..],
            }
        }
        Ok(())
    }
}
impl Writable for u8 {
    fn written_len(&self) -> usize {
        1
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.to_be_bytes().write(write).await
    }
}
impl Writable for u16 {
    fn written_len(&self) -> usize {
        2
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.to_be_bytes().write(write).await
    }
}
impl Writable for u32 {
    fn written_len(&self) -> usize {
        4
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.to_be_bytes().write(write).await
    }
}
impl Writable for bool {
    fn written_len(&self) -> usize {
        wlen!(u8)
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        <Self as Into<u8>>::into(*self).write(write).await
    }
}
impl Writable for VarByteInt {
    fn written_len(&self) -> usize {
        match self.value() {
            0..=127 => 1,
            128..=16_383 => 2,
            16_384..=2_097_151 => 3,
            2_097_152..=Self::MAX_ENCODABLE => 4,
            _ => unreachable!(
                "Invariant, never occurs if VarByteInts are generated using From, TryFrom and new"
            ),
        }
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let mut x = self.value();
        let mut encoded_byte: u8;

        loop {
            encoded_byte = (x % 128) as u8;
            x /= 128;

            if x > 0 {
                encoded_byte |= 128;
            }
            encoded_byte.write(write).await?;

            if x == 0 {
                return Ok(());
            }
        }
    }
}
impl<'b> Writable for MqttBinary<'b> {
    fn written_len(&self) -> usize {
        self.0.len() + wlen!(u16)
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        let len = self.len();
        len.write(write).await?;

        self.0.write(write).await?;

        Ok(())
    }
}
impl<'b> Writable for MqttString<'b> {
    fn written_len(&self) -> usize {
        self.0.written_len()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.0.write(write).await
    }
}
impl<'b> Writable for TopicName<'b> {
    fn written_len(&self) -> usize {
        self.as_ref().written_len()
    }

    async fn write<W: Write>(&self, write: &mut W) -> Result<(), WriteError<W::Error>> {
        self.as_ref().write(write).await
    }
}

pub(crate) use wlen;
