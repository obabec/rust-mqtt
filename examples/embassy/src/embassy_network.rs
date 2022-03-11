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
use core::ops::Range;
use drogue_device::traits::ip::{IpAddress, IpAddressV4, IpProtocol, SocketAddress};
use rust_mqtt::network::network_trait::Network;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;

use drogue_device::traits::tcp;
use drogue_device::traits::tcp::TcpStack;
use embedded_hal_async::delay::DelayUs;


pub struct DrogueNetwork<T, D> {
    ip: IpAddressV4,
    port: u16,
    socket: Option<T>,
    timer: Option<D>
}

impl<T, D> DrogueNetwork<T, D>
where
    T: TcpStack,
    D: DelayUs
{
    fn new(ip: [u8; 4], port: u16, stack: T, timer: D) -> Self {
        Self {
            ip: IpAddressV4::new(ip[0], ip[1], ip[2], ip[3]),
            port,
            socket: Some(stack),
            timer: Some(timer)
        }
    }
}

impl<T: TcpStack, D: DelayUs> Network for DrogueNetwork<T, D>
where
    T: TcpStack,
    D: DelayUs
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
    = impl Future<Output = Result<(), ReasonCode>> + 'm;

    fn new(ip: [u8; 4], port: u16) -> Self {
        Self {
            ip: IpAddressV4::new(ip[0], ip[1], ip[2], ip[3]),
            port,
            socket: None,
            timer: None
        }
    }

    fn create_connection(&'m mut self) -> Self::ConnectionFuture<'m> {
        async move {
            return if let Some(ref mut stack) = self.socket {
                stack.connect(stack.handle, IpProtocol::Tcp, SocketAddress::new(IpAddress::V4(self.ip), self.port))
            } else {
                Err(ReasonCode::NetworkError)
            }
        }
    }

    fn send(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m> {
        async move {
            return if let Some(ref mut stack) = self.socket {
                stack.write(stack.handle, &buffer[0..len])
                    .await
                    .map_err(|x| ReasonCode::NetworkError)
                    .map(())
            } else {
                Err(ReasonCode::NetworkError)
            }
        }
    }

    fn receive(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m> {
        async move {
            return if let Some(ref mut stack) = self.socket {
                stack.read(stack.handle, buffer)
                    .await
            } else {
                Err(ReasonCode::NetworkError)
            }
        }
    }

    fn count_down(&'m mut self, time_in_secs: u64) -> Self::TimerFuture<'m> {
        async move {
            return if let Some(ref mut time) = self.timer {
                time.delay_ms(time_in_secs as u32 * 1000)
                    .await
            } else {
                Err(ReasonCode::TimerNotSupported)
            }
        }
    }
}
