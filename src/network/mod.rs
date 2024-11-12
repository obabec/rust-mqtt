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

use crate::packet::v5::reason_codes::ReasonCode;
use embedded_io_async::{Read, ReadReady, Write};

pub struct NetworkConnection<T>
where
    T: Read + ReadReady + Write,
{
    io: T,
}

/// Network connection represents an established TCP connection.
impl<T> NetworkConnection<T>
where
    T: Read + ReadReady + Write,
{
    /// Create a new network handle using the provided IO implementation.
    pub fn new(io: T) -> Self {
        Self { io }
    }

    /// Send the data from `buffer` via TCP connection.
    pub async fn send(&mut self, buffer: &[u8]) -> Result<(), ReasonCode> {
        self.io
            .write_all(buffer)
            .await
            .map_err(|_| ReasonCode::NetworkError)?;

        self.io
            .flush()
            .await
            .map_err(|_| ReasonCode::NetworkError)?;

        Ok(())
    }

    /// Receive data to the `buffer` from TCP connection.
    pub async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, ReasonCode> {
        self.io
            .read(buffer)
            .await
            .map_err(|_| ReasonCode::NetworkError)
    }

    /// Get whether a TCP connection reader is ready.
    pub fn read_ready(&mut self) -> Result<bool, ReasonCode> {
        self.io.read_ready()
            .map_err(|_| ReasonCode::NetworkError)
    }

}
