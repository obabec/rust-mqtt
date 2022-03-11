/*/*
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

use alloc::format;
use alloc::string::String;
use core::future::Future;
use core::time::Duration;
use drogue_device::traits::ip::IpAddressV4;

use rust_mqtt::network::network_trait::Network;
use rust_mqtt::packet::v5::reason_codes::ReasonCode;

pub struct DrogueNetwork {
    ip: IpAddressV4,
    port: u16,
    socket: Option<SocketHandle>,
}

impl DrogueNetwork {
}

impl Embassy {}

impl Network for DrogueNetwork {
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
        Self {
            ip: IpAddressV4(ip[0], ip[1], ip[2], ip[3]),
            port,
            socket: None
        }
    }

    fn create_connection(&'m mut self) -> Self::ConnectionFuture<'m> {
        todo!()
    }

    fn send(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m> {
        todo!()
    }

    fn receive(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m> {
        todo!()
    }

    fn count_down(&'m mut self, time_in_secs: u64) -> Self::TimerFuture<'m> {
        todo!()
    }
}
*/