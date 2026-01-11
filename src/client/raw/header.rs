use core::hint::unreachable_unchecked;

use crate::{
    eio::Read,
    fmt::trace,
    header::{FixedHeader, PacketType},
    io::err::ReadError,
    types::VarByteInt,
};

#[derive(Debug)]
pub struct HeaderState {
    buffer: [u8; 5],
    read: u8,
}

impl HeaderState {
    pub fn new() -> Self {
        Self {
            buffer: [0; 5],
            read: 0,
        }
    }

    /// Cancel-safe
    ///
    /// Attempts to complete the header by reading a single byte
    ///
    /// Resets the state before returning
    pub async fn update<R: Read>(
        &mut self,
        r: &mut R,
    ) -> Result<Option<FixedHeader>, ReadError<R::Error>> {
        let i = self.read as usize;
        if i > 4 {
            // Safety: `self.read` gets reset to 0 when reaching 5
            unsafe { unreachable_unchecked() }
        }

        // Since i is in the 0..=4 range, we can safely index into `self.buffer`

        trace!("receiving byte {} of header", i);

        let read = r
            .read(&mut self.buffer[i..(i + 1)])
            .await
            .map_err(ReadError::Read)? as u8;

        match read {
            0 => return Err(ReadError::UnexpectedEOF),
            1 => self.read += 1,
            // Safety: `Read` can never return a value greater then the length of the slice.
            _ => unsafe { unreachable_unchecked() },
        }

        trace!("received {} byte(s) in total", self.read);

        if i == 0 {
            return if PacketType::from_type_and_flags(self.buffer[i]).is_err() {
                self.read = 0;
                Err(ReadError::MalformedPacket)
            } else {
                Ok(None)
            };
        }

        let is_continuation_byte = self.buffer[i] >= 128;

        if is_continuation_byte {
            if i == 4 {
                self.read = 0;
                Err(ReadError::MalformedPacket)
            } else {
                Ok(None)
            }
        } else {
            let slice = &self.buffer[1..=i];

            // Invariant: We checked that the slice is within the valid length range and
            // that the last byte matches the end condition of the variable byte integer encoding
            let remaining_len = VarByteInt::from_slice_unchecked(slice);

            self.read = 0;
            Ok(Some(FixedHeader {
                type_and_flags: self.buffer[0],
                remaining_len,
            }))
        }
    }
}

#[cfg(test)]
mod unit {
    use core::time::Duration;

    use embedded_io_adapters::tokio_1::FromTokio;
    use tokio::{
        io::{AsyncWriteExt, duplex},
        join,
        time::sleep,
    };
    use tokio_test::{assert_err, assert_ok};

    use crate::{
        client::raw::header::HeaderState, header::FixedHeader, io::err::ReadError,
        test::read::SliceReader, types::VarByteInt,
    };

    #[tokio::test]
    #[test_log::test]
    async fn minimal_at_once() {
        let (c, mut s) = duplex(64);
        let mut r = FromTokio::new(c);

        let tx = async {
            assert_ok!(s.write_all(&[0x10, 0x00]).await);
        };
        let rx = async {
            let mut header_state = HeaderState::new();

            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());

            let h = assert_ok!(header_state.update(&mut r).await);
            let h = assert_ok!(h.ok_or(()));
            assert_eq!(
                h,
                FixedHeader {
                    type_and_flags: 0x10,
                    remaining_len: VarByteInt::from(0u8)
                }
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn minimal_with_pause() {
        let (c, mut s) = duplex(64);
        let mut r = FromTokio::new(c);

        let tx = async {
            assert_ok!(s.write_u8(0x10).await);
            sleep(Duration::from_millis(100)).await;
            assert_ok!(s.write_u8(0x00).await);
        };
        let rx = async {
            let mut header_state = HeaderState::new();

            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);

            let h = assert_ok!(h.ok_or(()));
            assert_eq!(
                h,
                FixedHeader {
                    type_and_flags: 0x10,
                    remaining_len: VarByteInt::from(0u8)
                }
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn maximal_at_once() {
        let (c, mut s) = duplex(64);
        let mut r = FromTokio::new(c);

        let tx = async {
            assert_ok!(s.write_all(&[0x10, 0xFF, 0xFF, 0xFF, 0x7F]).await);
        };
        let rx = async {
            let mut header_state = HeaderState::new();

            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);

            let h = assert_ok!(h.ok_or(()));
            assert_eq!(
                h,
                FixedHeader {
                    type_and_flags: 0x10,
                    remaining_len: VarByteInt::try_from(VarByteInt::MAX_ENCODABLE).unwrap(),
                }
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn multiple_headers() {
        let (c, mut s) = duplex(64);
        let mut r = FromTokio::new(c);

        let tx = async {
            assert_ok!(s.write_all(&[0x10, 0xFF, 0xFF, 0x7F]).await);
            sleep(Duration::from_millis(100)).await;
            assert_ok!(s.write_all(&[0x89, 0x6E]).await);
            sleep(Duration::from_millis(100)).await;
            assert_ok!(s.write_all(&[0xA0, 0xFF, 0x7F]).await);
        };
        let rx = async {
            let mut header_state = HeaderState::new();

            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);

            let h = assert_ok!(h.ok_or(()));
            assert_eq!(
                h,
                FixedHeader {
                    type_and_flags: 0x10,
                    remaining_len: VarByteInt::try_from(2_097_151u32).unwrap(),
                }
            );

            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);

            let h = assert_ok!(h.ok_or(()));
            assert_eq!(
                h,
                FixedHeader {
                    type_and_flags: 0x89,
                    remaining_len: VarByteInt::from(110u8),
                }
            );

            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);

            let h = assert_ok!(h.ok_or(()));
            assert_eq!(
                h,
                FixedHeader {
                    type_and_flags: 0xA0,
                    remaining_len: VarByteInt::from(16_383u16),
                }
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn eof() {
        {
            let mut r = SliceReader::new(&[]);
            let mut header_state = HeaderState::new();
            let e = assert_err!(header_state.update(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }
        {
            let mut r = SliceReader::new(&[0x10]);
            let mut header_state = HeaderState::new();
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let e = assert_err!(header_state.update(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }
        {
            let mut r = SliceReader::new(&[0x20, 0x80]);
            let mut header_state = HeaderState::new();
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let e = assert_err!(header_state.update(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }
        {
            let mut r = SliceReader::new(&[0x30, 0x95, 0xF3]);
            let mut header_state = HeaderState::new();
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let e = assert_err!(header_state.update(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }
        {
            let mut r = SliceReader::new(&[0x40, 0xB3, 0xE2, 0xC0]);
            let mut header_state = HeaderState::new();
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let h = assert_ok!(header_state.update(&mut r).await);
            assert!(h.is_none());
            let e = assert_err!(header_state.update(&mut r).await);
            assert_eq!(e, ReadError::UnexpectedEOF);
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn reserved_packet_type() {
        let mut r = SliceReader::new(&[0x00]);
        let mut header_state = HeaderState::new();
        let e = assert_err!(header_state.update(&mut r).await);
        assert_eq!(e, ReadError::MalformedPacket);
    }

    #[tokio::test]
    #[test_log::test]
    async fn malformed_remaining_length() {
        let mut r = SliceReader::new(&[0x50, 0x80, 0x80, 0x80, 0x80]);
        let mut header_state = HeaderState::new();
        let h = assert_ok!(header_state.update(&mut r).await);
        assert!(h.is_none());
        let h = assert_ok!(header_state.update(&mut r).await);
        assert!(h.is_none());
        let h = assert_ok!(header_state.update(&mut r).await);
        assert!(h.is_none());
        let h = assert_ok!(header_state.update(&mut r).await);
        assert!(h.is_none());
        let e = assert_err!(header_state.update(&mut r).await);
        assert_eq!(e, ReadError::MalformedPacket);
    }
}
