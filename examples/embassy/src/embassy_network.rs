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
use drogue_device::actors::net::ConnectionFactory;
use drogue_device::actors::socket::Socket;
use drogue_device::actors::tcp::TcpActor;
use drogue_device::Address;
use drogue_device::traits::ip::{IpAddress, IpAddressV4, IpProtocol, SocketAddress};
use rust_mqtt::packet::v5::reason_codes::ReasonCode;

use drogue_device::traits::tcp;
use drogue_device::traits::tcp::TcpStack;
use embassy::time::Delay;
use embedded_hal_async::delay::DelayUs;
use rust_mqtt::network::network_trait::{NetworkConnection, NetworkConnectionFactory};


pub struct DrogueNetwork<A, D>
where
    A: TcpActor + 'static{
    socket: Option<Socket<A>>,
    timer: Option<D>
}

impl<A, D> DrogueNetwork<A, D>
where
    A: TcpActor + 'static,
    D: DelayUs
{
    pub fn new(socket: Socket<A>, timer: D) -> Self {
        Self {
            socket: Some(socket),
            timer: Some(timer)
        }
    }
}

impl<A, D> NetworkConnection for DrogueNetwork<A, D>
where
    A: TcpActor + 'static,
    D: DelayUs
{
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

    fn send(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m> {
        async move {
            return if let Some(ref mut connection) = self.socket {
                connection.write(&buffer[0..len])
                    .await
                    .map_err(|_| ReasonCode::NetworkError)
                    .map(|_| ())
            } else {
                Err(ReasonCode::NetworkError)
            }
        }
    }

    fn receive(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m> {
        async move {
            return if let Some(ref mut connection) = self.socket {
                connection.read(buffer)
                    .await
                    .map_err(|_| ReasonCode::NetworkError)
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
                    .map_err(|_| ReasonCode::TimerNotSupported)
                    .unwrap()
            }
        }
    }
}

pub struct DrogueConnectionFactory<A, D>
where
    A: TcpActor + 'static,
    D: DelayUs
{
    network: Address<A>,
    delay: D
}

impl<A, D> DrogueConnectionFactory<A, D>
where
    A: TcpActor + 'static,
    D: DelayUs
{
    pub fn new(network: Address<A>, delay: D) -> Self {
        Self {
            network,
            delay
        }
    }
}

impl<A, D> NetworkConnectionFactory for DrogueConnectionFactory<A, D>
where
    A: TcpActor + 'static,
    D: DelayUs
{
    type Connection = DrogueNetwork<A, D>;

    type ConnectionFuture<'m>
        where
            Self: 'm,
    = impl Future<Output = Result<Self::Connection, ReasonCode>> + 'm;

    fn connect<'m>(&'m mut self, ip: [u8; 4], port: u16) -> Self::ConnectionFuture<'m> {
        async move {
            let mut socket = Socket::new(self.network.clone(), self.network.open().await.unwrap());

            match socket
                .connect(IpProtocol::Tcp, SocketAddress::new(IpAddress::new_v4(ip[0],
                                                               ip[1], ip[2], ip[3]), port))
                .await
            {
                Ok(_) => {
                    log::trace!("Connection established");
                    Ok(DrogueNetwork::new(socket, &self.delay))
                }
                Err(e) => {
                    log::warn!("Error creating connection: {:?}", e);
                    socket.close().await.map_err(|e| ReasonCode::NetworkError)?;
                    Err(ReasonCode::NetworkError)
                }
            }
        }
    }
}