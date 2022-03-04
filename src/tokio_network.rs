use alloc::format;
use alloc::string::String;
use core::future::Future;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::network::network_trait::Network;
use crate::packet::reason_codes::ReasonCode;

pub struct TokioNetwork {
    ip: [u8; 4],
    port: u16,
    socket: Option<TcpStream>,
}

impl TokioNetwork {
    fn convert_ip(&mut self) -> String {
        String::from(format!(
            "{}.{}.{}.{}:{}",
            self.ip[0], self.ip[1], self.ip[2], self.ip[3], self.port
        ))
    }
}

impl TokioNetwork {}

impl Network for TokioNetwork {
    type ConnectionFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ReasonCode>> + 'm;
    type WriteFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ReasonCode>> + 'm;
    type ReadFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<usize, ReasonCode>> + 'm;

    fn new(ip: [u8; 4], port: u16) -> Self {
        return Self {
            ip,
            port,
            socket: Option::None,
        };
    }

    fn create_connection<'m>(&'m mut self) -> Self::ConnectionFuture<'m> {
        async move {
            TcpStream::connect(self.convert_ip())
                .await
                .map(|socket| self.socket = Some(socket))
                .map(|_| ())
                .map_err(|_| ReasonCode::NetworkError)
        }
    }

    fn send<'m>(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m> {
        async move {
            return if let Some(ref mut stream) = self.socket {
                stream
                    .write_all(&buffer[0..len])
                    .await
                    .map_err(|_| ReasonCode::NetworkError)
            } else {
                Err(ReasonCode::NetworkError)
            };
        }
    }

    fn receive<'m>(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m> {
        async move {
            return if let Some(ref mut stream) = self.socket {
                stream
                    .read(buffer)
                    .await
                    .map_err(|_| ReasonCode::NetworkError)
            } else {
                Err(ReasonCode::NetworkError)
            };
        }
    }
}
