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

use core::future::Future;

use crate::packet::v5::reason_codes::ReasonCode;

#[derive(Debug)]
pub enum NetworkError {
    Connection,
    Unknown,
    QoSAck,
    IDNotMatchedOnAck,
    NoMatchingSubs,
}
/// NetworkConnectionFactory implementation should create a TCP connection and return
/// the `Connection` trait implementation. Otherwise return `ReasonCode`.
pub trait NetworkConnectionFactory: Sized {
    type Connection: NetworkConnection;

    type ConnectionFuture<'m>: Future<Output = Result<Self::Connection, ReasonCode>>
    where
        Self: 'm;

    /// Connect function estabilish TCP connection and return the `Connection`.
    fn connect<'m>(&'m mut self, ip: [u8; 4], port: u16) -> Self::ConnectionFuture<'m>;
}

/// Network connection represents estabilished TCP connection created with `NetworkConnectionFactory`.
pub trait NetworkConnection {
    type SendFuture<'m>: Future<Output = Result<(), ReasonCode>>
    where
        Self: 'm;

    type ReceiveFuture<'m>: Future<Output = Result<usize, ReasonCode>>
    where
        Self: 'm;

    type CloseFuture<'m>: Future<Output = Result<(), ReasonCode>>;

    /// Send function should enable sending the data from `buffer` via TCP connection.
    fn send<'m>(&'m mut self, buffer: &'m [u8]) -> Self::SendFuture<'m>;

    /// Receive should enable receiving data to the `buffer` from TCP connection.
    fn receive<'m>(&'m mut self, buffer: &'m mut [u8]) -> Self::ReceiveFuture<'m>;

    /// Close function should close the TCP connection.
    fn close<'m>(self) -> Self::CloseFuture<'m>;
}
