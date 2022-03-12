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


pub trait NetworkConnectionFactory: Sized {
    type Connection: NetworkConnection;

    type ConnectionFuture<'m>: Future<Output = Result<Self::Connection, ReasonCode>>
    where
    Self: 'm;

    fn connect<'m>(&'m mut self, ip: [u8; 4], port: u16) -> Self::ConnectionFuture<'m>;
}


pub trait NetworkConnection {
    type WriteFuture<'m>: Future<Output = Result<(), ReasonCode>>
    where
        Self: 'm;

    type ReadFuture<'m>: Future<Output = Result<usize, ReasonCode>>
    where
        Self: 'm;

    type CloseFuture<'m>: Future<Output = Result<(), ReasonCode>>
    where
    Self: 'm;

    fn send(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m>;

    fn receive(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m>;

    fn close(self) -> Self::CloseFuture<'m>;
}
