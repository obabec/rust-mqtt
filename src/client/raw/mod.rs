//! Implements primitives for handling connections along with sending and receiving packets.

mod err;
mod header;
mod net;

pub use err::Error as RawError;
pub use net::Error as NetStateError;

use crate::{
    buffer::BufferProvider,
    client::raw::{header::HeaderState, net::NetState},
    eio::{Error, ErrorKind},
    fmt::{debug_assert, unreachable},
    header::FixedHeader,
    io::{err::WriteError, net::Transport, read::BodyReader},
    packet::{RxError, RxPacket, TxError, TxPacket},
    types::ReasonCode,
    v5::packet::DisconnectPacket,
};

/// An MQTT Client offering a low level api for sending and receiving packets
#[derive(Debug)]
pub(crate) struct Raw<'b, N: Transport, B: BufferProvider<'b>> {
    n: NetState<N>,
    buf: &'b mut B,
    header: HeaderState,
}

impl<'b, N: Transport, B: BufferProvider<'b>> Raw<'b, N, B> {
    pub fn new_disconnected(buf: &'b mut B) -> Self {
        Self {
            n: NetState::Terminated,
            buf,
            header: HeaderState::new(),
        }
    }

    pub fn set_net(&mut self, net: N) {
        debug_assert!(
            !self.n.is_ok(),
            "Network must not be in Ok() state to replace it."
        );
        self.n.replace(net);
    }

    pub fn buffer(&mut self) -> &mut B {
        self.buf
    }

    pub fn close_with(&mut self, reason_code: Option<ReasonCode>) {
        match reason_code {
            Some(r) => self.n.fail(r),
            None => {
                self.n.terminate();
            }
        }
    }

    /// Disconnect handler after an error occured.
    ///
    /// This expects the network to not be in Ok() state
    pub async fn abort(&mut self) -> Result<(), RawError<B::ProvisionError>> {
        debug_assert!(
            !self.n.is_ok(),
            "Network must not be in Ok() state to disconnect due to an error."
        );

        let r = self.n.get_reason_code();
        let mut n = self.n.terminate();

        match (&mut n, r) {
            (Some(n), Some(r)) => {
                let packet = DisconnectPacket::new(r);

                // Don't check whether length exceeds servers maximum packet size because we don't
                // add a reason string to the DISCONNECT packet -> length is always in the 4..=6 range in bytes.
                // The server really shouldn't reject this.
                packet.send(n).await.map_err(|e| match e {
                    TxError::Write(e) => RawError::Network(Error::kind(&e)),
                    TxError::WriteZero => RawError::Network(ErrorKind::WriteZero),
                })
            }
            (None, Some(_)) => unreachable!(
                "Netstate never contains a reason code when terminated and therefore not holding a network connection"
            ),
            (_, _) => Ok(()),
        }
    }

    fn handle_rx<E: Into<(RawError<B::ProvisionError>, Option<ReasonCode>)>>(
        &mut self,
        e: E,
    ) -> RawError<B::ProvisionError> {
        let (e, r) = e.into();

        match r {
            Some(r) => self.n.fail(r),
            None => {
                self.n.terminate();
            }
        }

        e
    }
    fn handle_tx<E: Into<RawError<B::ProvisionError>>>(
        &mut self,
        e: E,
    ) -> RawError<B::ProvisionError> {
        // Terminate right away because if send fails, sending another (DISCONNECT) packet doesn't make sense
        self.n.terminate();

        e.into()
    }

    /// Cancel-safe method to receive the fixed header of a packet
    pub async fn recv_header(&mut self) -> Result<FixedHeader, RawError<B::ProvisionError>> {
        let net = self.n.get()?;

        loop {
            match self.header.update(net).await {
                Ok(None) => {}
                Ok(Some(h)) => return Ok(h),
                Err(e) => {
                    let e: RxError<_, _> = e.into();
                    return Err(self.handle_rx(e));
                }
            }
        }
    }

    /// Not cancel-safe
    ///
    /// Does not perform a check on headers packet type
    /// => Assumes you call this only for correct packet headers
    pub async fn recv_body<P: RxPacket<'b>>(
        &mut self,
        header: &FixedHeader,
    ) -> Result<P, RawError<B::ProvisionError>> {
        let net = self.n.get()?;
        let reader = BodyReader::new(net, self.buf, header.remaining_len.size());

        P::receive(header, reader)
            .await
            .map_err(|e| self.handle_rx(e))
    }

    // pub async fn recv_full<P: RxPacket<'b>>(&mut self) -> Result<P, RawError<B::ProvisionError>> {
    //     let header = self.recv_header().await?;
    //     let packet_type = header.packet_type().map_err(|_r| {
    //         self.close_with(Some(ReasonCode::MalformedPacket));
    //         RawError::Server
    //     })?;
    //     if packet_type != P::PACKET_TYPE {
    //         self.close_with(Some(ReasonCode::ImplementationSpecificError));
    //         return Err(RawError::UnexpectedPacketType);
    //     }

    //     self.recv_body(&header).await
    // }

    pub async fn send<P: TxPacket>(
        &mut self,
        packet: &P,
    ) -> Result<(), RawError<B::ProvisionError>> {
        let net = self.n.get()?;

        packet.send(net).await.map_err(|e| self.handle_tx(e))
    }

    /// Cancel-safe if N::flush() is cancel-safe
    pub async fn flush(&mut self) -> Result<(), RawError<B::ProvisionError>> {
        self.n.get()?.flush().await.map_err(|e| {
            let e: WriteError<_> = e.into();
            let e: TxError<_> = e.into();
            self.handle_tx(e)
        })
    }
}

#[cfg(test)]
mod unit {
    use core::time::Duration;

    use embedded_io_adapters::tokio_1::FromTokio;
    use tokio::{
        io::{AsyncWriteExt, duplex},
        join,
        sync::oneshot::channel,
        time::{sleep, timeout},
    };
    use tokio_test::{assert_err, assert_ok};

    #[cfg(feature = "alloc")]
    use crate::buffer::AllocBuffer;
    #[cfg(feature = "bump")]
    use crate::buffer::BumpBuffer;

    use crate::{
        client::raw::Raw,
        header::{FixedHeader, PacketType},
        types::VarByteInt,
    };

    #[tokio::test]
    #[test_log::test]
    async fn recv_header_simple() {
        #[cfg(feature = "alloc")]
        let mut b = AllocBuffer;
        #[cfg(feature = "bump")]
        let mut b = [0; 64];
        #[cfg(feature = "bump")]
        let mut b = BumpBuffer::new(&mut b);
        let (c, mut s) = duplex(64);
        let r = FromTokio::new(c);

        let mut c = Raw::new_disconnected(&mut b);
        c.set_net(r);

        let tx = async {
            assert_ok!(s.write_all(&[0x10, 0x00, 0x24]).await);
        };
        let rx = async {
            let h = assert_ok!(c.recv_header().await);
            assert_eq!(
                h,
                FixedHeader::new(PacketType::Connect, 0x00, VarByteInt::from(0u8))
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn recv_header_with_pause() {
        #[cfg(feature = "alloc")]
        let mut b = AllocBuffer;
        #[cfg(feature = "bump")]
        let mut b = [0; 64];
        #[cfg(feature = "bump")]
        let mut b = BumpBuffer::new(&mut b);
        let (c, mut s) = duplex(64);
        let r = FromTokio::new(c);

        let mut c = Raw::new_disconnected(&mut b);
        c.set_net(r);

        let tx = async {
            assert_ok!(s.write_u8(0xE0).await);
            sleep(Duration::from_millis(100)).await;
            assert_ok!(s.write_u8(0x80).await);
            sleep(Duration::from_millis(100)).await;
            assert_ok!(s.write_u8(0x80).await);
            sleep(Duration::from_millis(100)).await;
            assert_ok!(s.write_u8(0x01).await);
        };
        let rx = async {
            let h = assert_ok!(c.recv_header().await);
            assert_eq!(
                h,
                FixedHeader::new(PacketType::Disconnect, 0x00, VarByteInt::from(16_384u16))
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn recv_header_cancel_no_progres() {
        #[cfg(feature = "alloc")]
        let mut b = AllocBuffer;
        #[cfg(feature = "bump")]
        let mut b = [0; 64];
        #[cfg(feature = "bump")]
        let mut b = BumpBuffer::new(&mut b);
        let (c, mut s) = duplex(64);
        let r = FromTokio::new(c);
        let (rx_ready, tx_ready) = channel();

        let mut c = Raw::new_disconnected(&mut b);
        c.set_net(r);

        let tx = async {
            assert_ok!(tx_ready.await);
            assert_ok!(s.write_all(&[0xE0, 0x00]).await);
        };
        let rx = async {
            assert_err!(timeout(Duration::from_millis(100), c.recv_header()).await);
            assert_ok!(rx_ready.send(()));

            let h = assert_ok!(c.recv_header().await);
            assert_eq!(
                h,
                FixedHeader::new(PacketType::Disconnect, 0x00, VarByteInt::from(0u8))
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn recv_header_cancel_type_and_flags_byte() {
        #[cfg(feature = "alloc")]
        let mut b = AllocBuffer;
        #[cfg(feature = "bump")]
        let mut b = [0; 64];
        #[cfg(feature = "bump")]
        let mut b = BumpBuffer::new(&mut b);
        let (c, mut s) = duplex(64);
        let r = FromTokio::new(c);
        let (rx_ready, tx_ready) = channel();

        let mut c = Raw::new_disconnected(&mut b);
        c.set_net(r);

        let tx = async {
            assert_ok!(s.write_u8(0xA0).await);
            assert_ok!(tx_ready.await);
            assert_ok!(s.write_all(&[0x80, 0x80, 0x80, 0x01]).await);
        };
        let rx = async {
            assert_err!(timeout(Duration::from_millis(100), c.recv_header()).await);
            assert_ok!(rx_ready.send(()));

            let h = assert_ok!(c.recv_header().await);
            assert_eq!(
                h,
                FixedHeader::new(
                    PacketType::Unsubscribe,
                    0x00,
                    VarByteInt::try_from(2_097_152u32).unwrap()
                )
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn recv_header_cancel_single_length_byte() {
        #[cfg(feature = "alloc")]
        let mut b = AllocBuffer;
        #[cfg(feature = "bump")]
        let mut b = [0; 64];
        #[cfg(feature = "bump")]
        let mut b = BumpBuffer::new(&mut b);
        let (c, mut s) = duplex(64);
        let r = FromTokio::new(c);
        let (rx_ready, tx_ready) = channel();

        let mut c = Raw::new_disconnected(&mut b);
        c.set_net(r);

        let tx = async {
            assert_ok!(s.write_all(&[0xD7, 0xFF]).await);
            assert_ok!(tx_ready.await);
            assert_ok!(s.write_all(&[0xFF, 0xFF, 0x7F]).await);
        };
        let rx = async {
            assert_err!(timeout(Duration::from_millis(100), c.recv_header()).await);
            assert_ok!(rx_ready.send(()));

            let h = assert_ok!(c.recv_header().await);
            assert_eq!(
                h,
                FixedHeader::new(
                    PacketType::Pingresp,
                    0x07,
                    VarByteInt::try_from(VarByteInt::MAX_ENCODABLE).unwrap()
                )
            );
        };

        join!(rx, tx);
    }

    #[tokio::test]
    #[test_log::test]
    async fn recv_header_cancel_multi() {
        #[cfg(feature = "alloc")]
        let mut b = AllocBuffer;
        #[cfg(feature = "bump")]
        let mut b = [0; 64];
        #[cfg(feature = "bump")]
        let mut b = BumpBuffer::new(&mut b);
        let (c, mut s) = duplex(64);
        let r = FromTokio::new(c);
        let (rx_ready1, tx_ready1) = channel();
        let (rx_ready2, tx_ready2) = channel();
        let (rx_ready3, tx_ready3) = channel();

        let mut c = Raw::new_disconnected(&mut b);
        c.set_net(r);

        let tx = async {
            assert_ok!(s.write_u8(0x68).await);
            assert_ok!(tx_ready1.await);
            assert_ok!(s.write_u8(0xFF).await);
            assert_ok!(tx_ready2.await);
            assert_ok!(s.write_u8(0xFF).await);
            assert_ok!(tx_ready3.await);
            assert_ok!(s.write_u8(0x7F).await);
        };
        let rx = async {
            assert_err!(timeout(Duration::from_millis(50), c.recv_header()).await);
            assert_ok!(rx_ready1.send(()));
            assert_err!(timeout(Duration::from_millis(50), c.recv_header()).await);
            assert_ok!(rx_ready2.send(()));
            assert_err!(timeout(Duration::from_millis(50), c.recv_header()).await);
            assert_ok!(rx_ready3.send(()));

            let h = assert_ok!(c.recv_header().await);
            assert_eq!(
                h,
                FixedHeader::new(
                    PacketType::Pubrel,
                    0x08,
                    VarByteInt::try_from(2_097_151u32).unwrap()
                )
            );
        };

        join!(rx, tx);
    }
}
