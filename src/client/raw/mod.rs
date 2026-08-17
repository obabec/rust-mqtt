//! Implements primitives for handling connections along with sending and receiving packets.

mod err;
mod header;
mod net;

use core::matches;

pub use err::AbortError;
pub(crate) use err::Error as RawError;
pub(crate) use net::Error as NetStateError;

use heapless::Vec;

#[cfg(debug_assertions)]
use crate::fmt::unreachable;
use crate::{
    buffer::BufferProvider,
    client::raw::{header::HeaderState, net::NetState},
    fmt::{debug, debug_assert, error, warn},
    header::FixedHeader,
    io::{Transport, err::WriteError, read::BodyReader},
    packet::{RxError, RxPacket, TxError, TxPacket},
    types::ReasonCode,
    v5::packet::DisconnectPacket,
};

/// An MQTT Client offering a low level api for sending and receiving packets
pub(crate) struct Raw<'b, N: Transport, B: BufferProvider<'b>> {
    n: NetState<N>,
    buf: &'b mut B,
    header: HeaderState,
}

impl<'b, N: Transport, B: BufferProvider<'b>> core::fmt::Debug for Raw<'b, N, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Raw")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "defmt")]
impl<'b, N: Transport, B: BufferProvider<'b>> defmt::Format for Raw<'b, N, B> {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "Raw {{ header: {:?}, .. }}", self.header);
    }
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
        self.n.replace(net);
    }

    pub fn buffer(&self) -> &B {
        self.buf
    }

    pub fn buffer_mut(&mut self) -> &mut B {
        self.buf
    }

    pub fn prepare_disconnect(&mut self, reason_code: ReasonCode) {
        debug_assert!(self.n.is_ok());

        self.n.fail(reason_code);
    }

    pub fn prepare_close(&mut self) {
        debug_assert!(
            matches!(self.n, NetState::Ok(_) | NetState::DueDisconnect(_, _)),
            "transport layer must be in a working state to close it."
        );

        self.n.deactivate();
    }

    /// Disconnect handler after an error occured.
    ///
    /// This expects the network to be in neither `Ok(N)` nor `Terminated` state
    pub async fn abort(&mut self) -> Result<N, AbortError> {
        debug_assert!(
            !self.n.is_terminated(),
            "network must be in DueDisconnect(N, ReasonCode) or Inactive(N) state to disconnect due to an error."
        );

        // We want to ensure correct operability (e.g. subsequent calls to `Client::connect`) even when this
        // future is dropped. Therefore we remove the network from the client handle before any await point.
        // In case of a healthy connection, we set the network connection back into our netstate before the
        // first await point.
        let n = self.n.terminate();

        match n {
            NetState::Ok(n) => {
                warn!("abort() called during healthy MQTT connection");

                self.n.replace(n);

                Err(AbortError::Connected)
            }
            NetState::Terminated => {
                warn!("abort() called when no network connection is present");

                Err(AbortError::Terminated)
            }
            NetState::DueDisconnect(mut n, r) => {
                let packet = DisconnectPacket::<0>::new(r, None, None, Vec::new());

                debug!("sending DISCONNECT packet with reason code: {:?}", r);

                // Don't check whether length exceeds servers maximum packet size because we don't
                // add properties to the DISCONNECT packet -> length is always in the 4..=6 range in bytes.
                // The server really shouldn't reject this.
                if let Err(e) = packet.send(&mut n).await {
                    error!(
                        "I/O error during send: {:?}",
                        <_ as Into<RawError<B::ProvisionError>>>::into(e)
                    )
                };

                Ok(n)
            }
            NetState::Inactive(n) => Ok(n),
        }
    }

    fn handle_rx<E: Into<(RawError<B::ProvisionError>, Option<ReasonCode>)>>(
        &mut self,
        e: E,
    ) -> RawError<B::ProvisionError> {
        let (e, r) = e.into();

        match e {
            RawError::Network(ref e) => error!("I/O error during receive: {:?}", e),
            RawError::Disconnected => {
                #[cfg(debug_assertions)]
                unreachable!(
                    "only instantiated from `NetStateError` which is not handled with `handle_rx` and logged separately"
                );
                #[cfg(not(debug_assertions))]
                error!(
                    "unreachable: only instantiated from `NetStateError` which is not handled with `handle_rx` and logged separately"
                );
            }
            RawError::Alloc(ref e) => error!("buffer provision failed: {:?}", e),
            RawError::Server => error!("server protocol violation"),
        }

        match r {
            Some(reason_code) => self.prepare_disconnect(reason_code),
            None => self.n.deactivate(),
        }

        e
    }
    fn handle_tx<E: Into<RawError<B::ProvisionError>>>(
        &mut self,
        e: E,
    ) -> RawError<B::ProvisionError> {
        let e = e.into();

        match e {
            RawError::Network(ref e) => error!("I/O error during send: {:?}", e),
            RawError::Disconnected => {
                #[cfg(debug_assertions)]
                unreachable!(
                    "only instantiated from `NetStateError` which is not handled with `handle_tx` and logged separately"
                );
                #[cfg(not(debug_assertions))]
                error!(
                    "unreachable: only instantiated from `NetStateError` which is not handled with `handle_tx` and logged separately"
                );
            }
            RawError::Alloc(_) => {
                #[cfg(debug_assertions)]
                unreachable!("writing cannot trigger allocation");
                #[cfg(not(debug_assertions))]
                error!("unreachable: writing cannot trigger allocation");
            }
            RawError::Server => {
                #[cfg(debug_assertions)]
                unreachable!("server error cannot be caused by sending");
                #[cfg(not(debug_assertions))]
                error!("unreachable: server error cannot be caused by sending");
            }
        }

        // Deactivate right away because if send fails, sending a (DISCONNECT) packet doesn't make sense
        self.n.deactivate();

        e
    }

    /// Cancel-safe method to receive the fixed header of a packet
    pub async fn recv_header(&mut self) -> Result<FixedHeader, RawError<B::ProvisionError>> {
        let net = self.n.get().inspect_err(|e| match e {
            NetStateError::Faulted => {
                warn!("attempted to receive from a faulted mqtt connection")
            }
            NetStateError::Inactive => {
                warn!("attempted to receive from a faulted mqtt/network connection")
            }
            NetStateError::Terminated => {
                warn!("attempted to receive from a closed network connection")
            }
        })?;

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
        let net = self.n.get().inspect_err(|e| match e {
            NetStateError::Faulted => warn!("attempted to receive from a faulted mqtt connection"),
            NetStateError::Inactive => {
                warn!("attempted to receive from a faulted mqtt/network connection")
            }
            NetStateError::Terminated => {
                warn!("attempted to receive from a closed network connection")
            }
        })?;
        let reader = BodyReader::new(net, self.buf, header.remaining_len.size());

        P::receive(header, reader)
            .await
            .map_err(|e| self.handle_rx(e))
    }

    pub async fn send<P: TxPacket>(
        &mut self,
        packet: &P,
    ) -> Result<(), RawError<B::ProvisionError>> {
        let net = self.n.get().inspect_err(|e| match e {
            NetStateError::Faulted => warn!("attempted to send on a faulted mqtt connection"),
            NetStateError::Inactive => {
                warn!("attempted to send on a faulted mqtt/network connection")
            }
            NetStateError::Terminated => warn!("attempted to send on a closed network connection"),
        })?;
        packet.send(net).await.map_err(|e| self.handle_tx(e))
    }

    /// Cancel-safe if `N::flush()` is cancel-safe
    pub async fn flush(&mut self) -> Result<(), RawError<B::ProvisionError>> {
        let net = self.n.get().inspect_err(|e| match e {
            NetStateError::Faulted => warn!("attempted to flush a faulted mqtt connection"),
            NetStateError::Inactive => {
                warn!("attempted to flush a faulted mqtt/network connection")
            }
            NetStateError::Terminated => warn!("attempted to flush a closed network connection"),
        })?;

        net.flush().await.map_err(|e| {
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
                    VarByteInt::new(2_097_152u32).unwrap()
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
                    VarByteInt::new(VarByteInt::MAX_ENCODABLE).unwrap()
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
                    VarByteInt::new(2_097_151u32).unwrap()
                )
            );
        };

        join!(rx, tx);
    }
}
