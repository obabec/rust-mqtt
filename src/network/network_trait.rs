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

pub trait Network {
    type ConnectionFuture<'m>: Future<Output = Result<(), ReasonCode>>
    where
        Self: 'm;

    type WriteFuture<'m>: Future<Output = Result<(), ReasonCode>>
    where
        Self: 'm;

    type ReadFuture<'m>: Future<Output = Result<usize, ReasonCode>>
    where
        Self: 'm;

    type TimerFuture<'m>: Future<Output = ()>
    where
    Self: 'm;

    fn new(ip: [u8; 4], port: u16) -> Self;

    fn create_connection(&'m mut self) -> Self::ConnectionFuture<'m>;

    fn send(&'m mut self, buffer: &'m mut [u8], len: usize) -> Self::WriteFuture<'m>;

    fn receive(&'m mut self, buffer: &'m mut [u8]) -> Self::ReadFuture<'m>;

    fn count_down(&'m mut self, time_in_secs: u64) -> Self::TimerFuture<'m>;
}
