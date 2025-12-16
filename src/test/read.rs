use core::{cmp::min, fmt};

use crate::eio::{self, ErrorType, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceReader<'a> {
    slice: &'a [u8],
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    use tokio_test::assert_ok;

    use crate::{eio::Read, test::read::SliceReader};

    #[tokio::test]
    #[test_log::test]
    async fn read_full() {
        let mut reader = SliceReader::new(b"hello there");
        let mut buf = [0u8; 5];

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_partial() {
        let mut reader = SliceReader::new(b"hello");
        let mut buf = [0u8; 10];

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], b"hello");
        assert_eq!(&buf[n..], [0, 0, 0, 0, 0]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_chunks() {
        let mut reader = SliceReader::new(b"ab");
        let mut buf = [0u8; 1];

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 1);
        assert_eq!(&buf, b"a");

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 1);
        assert_eq!(&buf, b"b");
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_empty_eof() {
        let mut reader = SliceReader::new(b"");
        let mut buf = [0u8; 1];

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 0);
        assert_eq!(&buf[..], [0]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn read_some_eof() {
        let mut reader = SliceReader::new(b"hi");
        let mut buf = [0u8; 2];

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 2);
        assert_eq!(&buf[..], b"hi");

        let n = assert_ok!(reader.read(&mut buf).await);
        assert_eq!(n, 0);
        assert_eq!(&buf[..], b"hi");
    }
}
