/*
 * MIT License
 *
 * Copyright (c) [2022] [Ondrej Babec <ond.babec@gmail.com>]
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
extern crate alloc;
use alloc::format;
use alloc::string::String;
use core::future::Future;
use core::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;
use crate::network::network_trait::Network;
use crate::packet::v5::reason_codes::ReasonCode;

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

    type TimerFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = ()>;

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

    fn count_down(&'m mut self, time_in_secs: u64) -> Self::TimerFuture<'m> {
        async move {
            return sleep(Duration::from_secs(time_in_secs))
                .await
        }
    }
}
