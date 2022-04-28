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

use crate::network::{NetworkConnection, NetworkConnectionFactory};
use crate::packet::v5::reason_codes::ReasonCode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

pub struct TokioNetwork {
    stream: TcpStream,
}

impl TokioNetwork {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn convert_ip(ip: [u8; 4], port: u16) -> String {
        String::from(format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port))
    }
}

impl NetworkConnection for TokioNetwork {
    type SendFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ReasonCode>> + 'm;

    type ReceiveFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<usize, ReasonCode>> + 'm;

    type CloseFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<(), ReasonCode>> + 'm;

    /*type TimerFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = ()>;*/

    fn send<'m>(&'m mut self, buffer: &'m [u8]) -> Self::SendFuture<'m> {
        async move {
            self.stream
                .write_all(buffer)
                .await
                .map_err(|_| ReasonCode::NetworkError)
        }
    }

    fn receive<'m>(&'m mut self, buffer: &'m mut [u8]) -> Self::ReceiveFuture<'m> {
        async move {
            self.stream
                .read(buffer)
                .await
                .map_err(|_| ReasonCode::NetworkError)
        }
    }

    fn close<'m>(mut self) -> Self::CloseFuture<'m> {
        async move {
            self.stream
                .shutdown()
                .await
                .map_err(|_| ReasonCode::NetworkError)
        }
    }
}

pub struct TokioNetworkFactory {}

impl TokioNetworkFactory {
    pub fn new() -> Self {
        Self {}
    }
}

impl NetworkConnectionFactory for TokioNetworkFactory {
    type Connection = TokioNetwork;

    type ConnectionFuture<'m>
    where
        Self: 'm,
    = impl Future<Output = Result<TokioNetwork, ReasonCode>> + 'm;

    fn connect<'m>(&'m mut self, ip: [u8; 4], port: u16) -> Self::ConnectionFuture<'m> {
        async move {
            let stream = TcpStream::connect(TokioNetwork::convert_ip(ip, port))
                .await
                .map_err(|_| ReasonCode::NetworkError)?;
            Ok(TokioNetwork::new(stream))
        }
    }
}
