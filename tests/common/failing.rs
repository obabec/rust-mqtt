use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};

pub enum ByteLimit {
    Unlimited,
    FailAfter(usize),
}

pin_project! {
    pub struct FailingTcp {
        #[pin]
        tcp: TcpStream,

        read_limit: ByteLimit,
        write_limit: ByteLimit,

        read_seen: usize,
        write_seen: usize,
    }
}

impl FailingTcp {
    pub fn new(tcp: TcpStream, read_limit: ByteLimit, write_limit: ByteLimit) -> Self {
        Self {
            tcp,
            read_limit,
            write_limit,
            read_seen: 0,
            write_seen: 0,
        }
    }
}

impl AsyncRead for FailingTcp {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();

        let remaining = match *this.read_limit {
            ByteLimit::Unlimited => usize::MAX,
            ByteLimit::FailAfter(n) => n.saturating_sub(*this.read_seen),
        };

        if remaining == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "injected read failure",
            )));
        }

        let max = remaining.min(buf.remaining());

        // Create a temporary ReadBuf capped to `max`
        let unfilled = buf.initialize_unfilled_to(max);
        let mut limited = ReadBuf::new(unfilled);

        let before = limited.filled().len();
        let poll = this.tcp.poll_read(cx, &mut limited);
        let after = limited.filled().len();

        let read_now = after - before;
        *this.read_seen += read_now;
        buf.advance(read_now);

        poll
    }
}

impl AsyncWrite for FailingTcp {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();

        let remaining = match *this.write_limit {
            ByteLimit::Unlimited => usize::MAX,
            ByteLimit::FailAfter(n) => n.saturating_sub(*this.write_seen),
        };

        if remaining == 0 {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "injected write failure",
            )));
        }

        let cap = remaining.min(buf.len());
        let poll = this.tcp.poll_write(cx, &buf[..cap]);

        if let Poll::Ready(Ok(n)) = poll {
            *this.write_seen += n;
        }

        poll
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().tcp.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().tcp.poll_shutdown(cx)
    }
}
