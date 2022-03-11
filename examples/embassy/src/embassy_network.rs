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
use drogue_device::traits::ip::{IpAddress, IpAddressV4, IpProtocol, SocketAddress};
use rust_mqtt::network::network_trait::Network;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;

use drogue_device::traits::tcp;
use drogue_device::traits::tcp::TcpStack;
use embassy::io::{AsyncBufReadExt, AsyncWriteExt};
use embassy_traits::delay::Delay;

pub struct DrogueNetwork<T, D> {
    ip: IpAddressV4,
    port: u16,
    socket: Option<T>,
    timer: Option<D>
}

impl DrogueNetwork<T, D>
where
    T: TcpStack,
    D: Delay
{
    fn new(ip: [u8; 4], port: u16, stack: T, timer: D) -> Self {
        Self {
            ip: IpAddressV4(ip[0], ip[1], ip[2], ip[3]),
            port,
            socket: Some(stack),
            timer: Some(timer)
        }
    }
}

impl Network for DrogueNetwork<T, D>
where
    T: TcpStack,
    D: Delay
{
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
    = impl Future<Output = ()> + 'm;

    fn new(ip: [u8; 4], port: u16) -> Self {
        Self {
            ip: IpAddressV4(ip[0], ip[1], ip[2], ip[3]),
            port,
            socket: None,
            timer: None
        }
    }

    fn create_connection<T: TcpStack>(&'m mut self) -> Self::ConnectionFuture<'m> {
        async move {
            return if let Some(ref mut stack) = self.socket {
                stack.connect(stack, IpProtocol::Tcp, SocketAddress::new(IpAddress::V4(self.ip), self.port))
            } else {
                Err(ReasonCode::NetworkError);
            }
        }
    }

    fn send(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m> {
        async move {
            return if let Some(ref mut stack) = self.socket {
                stack.write(stack, &buffer[0..len])
            } else {
                Err(ReasonCode::NetworkError);
            }
        }
    }

    fn receive(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m> {
        async move {
            return if let Some(ref mut stack) = self.socket {
                stack.read(stack, & mut buffer[0..len])
            } else {
                Err(ReasonCode::NetworkError);
            }
        }
    }

    fn count_down(&'m mut self, time_in_secs: u64) -> Self::TimerFuture<'m> {
        async move {
            return if let Some(time) = self.timer {
                time.delay_ms(time_in_secs * 1000);
            } else {
                Err(ReasonCode::TimerNotSupported);
            }
        }
    }
}
