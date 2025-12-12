use core::{cmp::min, fmt};

use crate::eio::{self, ErrorType, Read};

pub struct SliceReader<'a> {
    slice: &'a [u8],
    index: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SliceReaderError;
impl fmt::Display for SliceReaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl core::error::Error for SliceReaderError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}
impl eio::Error for SliceReaderError {
    fn kind(&self) -> eio::ErrorKind {
        eio::ErrorKind::Other
    }
}
impl<'a> ErrorType for SliceReader<'a> {
    type Error = SliceReaderError;
}
impl<'a> Read for SliceReader<'a> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let left = self.slice.len() - self.index;
        let reading = min(left, buf.len());
        let end = self.index + reading;
        let src = &self.slice[self.index..end];
        let dest = &mut buf[..reading];

        dest.clone_from_slice(src);
        self.index = end;
        Ok(reading)
    }
}

impl<'a> SliceReader<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self { slice, index: 0 }
    }
}

#[cfg(test)]
mod unit {
    use crate::{eio::Read, test::read::SliceReader};

    #[tokio::test]
    #[test_log::test]
    async fn reads_in_one() {
        let mut reader = SliceReader::new(b"hello");
        let mut buf = [0u8; 5];

        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");

        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);
        assert_eq!(&buf, b"hello");
    }

    #[tokio::test]
    #[test_log::test]
    async fn reads_in_chunks() {
        let mut reader = SliceReader::new(b"ab");
        let mut buf = [0u8; 1];

        let n1 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n1, 1);
        assert_eq!(&buf, b"a");

        let n2 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n2, 1);
        assert_eq!(&buf, b"b");

        // No more data: should return 0 bytes read (EOF)
        let n3 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n3, 0);
        assert_eq!(&buf, b"b");
    }
}
