//! Implements primitives for handling connections along with sending and receiving packets.

mod err;
mod header;
mod net;

use core::marker::PhantomData;

pub use err::Error as RawError;
pub use net::Error as NetStateError;

use crate::{
    buffer::BufferProvider,
    client::raw::{header::HeaderState, net::NetState},
    eio::{Error, ErrorKind},
    fmt::{debug_assert, panic, unreachable},
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
    _b: PhantomData<&'b ()>,
}

impl<'b, N: Transport, B: BufferProvider<'b>> Raw<'b, N, B> {
    pub fn new_disconnected(buf: &'b mut B) -> Self {
        Self {
            n: NetState::Terminated,
            buf,
            header: HeaderState::new(),
            _b: PhantomData,
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

                packet.send(n).await.map_err(|e| match e {
                    TxError::Write(e) => RawError::Network(Error::kind(&e)),
                    TxError::WriteZero => RawError::Network(ErrorKind::WriteZero),
                    TxError::RemainingLenExceeded => panic!("DISCONNECT never exceeds max length"),
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
