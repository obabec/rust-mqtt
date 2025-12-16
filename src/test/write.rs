use core::{cmp::min, fmt};

use crate::eio::{self, ErrorType, Write};

pub struct SliceWriter<'a> {
    slice: &'a mut [u8],
    index: usize,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SliceWriterError;
impl fmt::Display for SliceWriterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl core::error::Error for SliceWriterError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        None
    }
}
impl eio::Error for SliceWriterError {
    fn kind(&self) -> eio::ErrorKind {
        eio::ErrorKind::Other
    }
}
impl<'a> ErrorType for SliceWriter<'a> {
    type Error = SliceWriterError;
}
impl<'a> Write for SliceWriter<'a> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let left = self.slice.len().saturating_sub(self.index);
        let writing = min(left, buf.len());
        let end = self.index + writing;
        if writing == 0 {
            return Ok(0);
        }

        let dest = &mut self.slice[self.index..end];
        let src = &buf[..writing];

        dest.clone_from_slice(src);
        self.index = end;
        Ok(writing)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> SliceWriter<'a> {
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self { slice, index: 0 }
    }

    pub fn written(&self) -> usize {
        self.index
    }
}

#[cfg(test)]
mod unit {
    use crate::{eio::Write, test::write::SliceWriter};
    use tokio_test::assert_ok;

    #[tokio::test]
    #[test_log::test]
    async fn writes_in_one() {
        let mut buf = [0u8; 5];
        {
            let mut writer = SliceWriter::new(&mut buf);

            let n = assert_ok!(writer.write(b"hello").await);
            assert_eq!(n, 5);

            let n = assert_ok!(writer.write(b"again").await);
            assert_eq!(n, 0);
        }

        assert_eq!(&buf, b"hello");
    }

    #[tokio::test]
    #[test_log::test]
    async fn writes_in_chunks() {
        let mut buf = [0u8; 2];
        {
            let mut writer = SliceWriter::new(&mut buf);

            let n1 = assert_ok!(writer.write(b"a").await);
            assert_eq!(n1, 1);

            let n2 = assert_ok!(writer.write(b"b").await);
            assert_eq!(n2, 1);

            let n3 = assert_ok!(writer.write(b"c").await);
            assert_eq!(n3, 0);
        }

        assert_eq!(&buf, b"ab");
    }

    #[tokio::test]
    #[test_log::test]
    async fn partial_write_when_not_enough_space() {
        let mut buf = [0u8; 3];
        {
            let mut writer = SliceWriter::new(&mut buf);

            let n1 = assert_ok!(writer.write(b"ab").await);
            assert_eq!(n1, 2);

            let n2 = assert_ok!(writer.write(b"xyz").await);
            assert_eq!(n2, 1);
        }

        assert_eq!(&buf, b"abx");
    }
}
