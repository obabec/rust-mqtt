use core::fmt::Error;

use core::future::Future;
use crate::packet::mqtt_packet::Packet;

pub enum NetworkError {
    Connection,
    Unknown,
}



pub trait Network {
    type ConnectionFuture<'m>: Future<Output = Result<(), NetworkError>>
    where
    Self: 'm;

    type WriteFuture<'m>: Future<Output = Result<(), NetworkError>>
    where
        Self: 'm;

    type ReadFuture<'m>: Future<Output = Result<usize, NetworkError>>
    where
        Self: 'm;

    fn new(ip: [u8; 4], port: u16) -> Self;

    fn create_connection(& mut self) -> Self::ConnectionFuture<'m>;

    fn send(& mut self, buffer: & mut [u8], len: usize) -> Self::WriteFuture<'m>;

    fn receive(& mut self, buffer: & mut [u8]) -> Self::ReadFuture<'m>;
}
