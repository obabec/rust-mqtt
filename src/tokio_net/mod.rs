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

use tokio::net::TcpStream;

use embedded_io::{
    adapters::FromTokio,
    asynch::{Read, Write},
    Io,
};
use embedded_nal_async::Close;

/// TokioNetwork is an implementation of the `NetworkConnection` trait. This implementation
/// allows communication through the `Tokio` TcpStream.
pub struct TokioNetwork {
    stream: FromTokio<TcpStream>,
}

impl Io for TokioNetwork {
    type Error = std::io::Error;
}

impl Read for TokioNetwork {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
        self.stream.read(buf)
    }
}

impl Write for TokioNetwork {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>>
    where
        Self: 'a;

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        self.stream.write(buf)
    }

    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        self.stream.flush()
    }
}

impl TokioNetwork {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: FromTokio::new(stream),
        }
    }

    pub fn convert_ip(ip: [u8; 4], port: u16) -> String {
        String::from(format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port))
    }
}

impl Close for TokioNetwork {
    type CloseFuture<'a> = impl Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;
    fn close<'a>(&'a mut self) -> Self::CloseFuture<'a> {
        async move {
            // Drop will do the close
            Ok(())
        }
    }
}
