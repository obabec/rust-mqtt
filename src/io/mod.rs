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

mod reader;
mod writer;
mod codec;

pub use reader::*;
pub use writer::*;
pub use codec::*;

use core::fmt::{Display, Formatter};

#[derive(core::fmt::Debug, Clone, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Utf8Error,
    InsufficientBufferSize,
    VariableByteIntegerError,
    IdNotFound,
    EncodingError,
    DecodingError,
    PacketTypeMismatch,
    WrongPacketToDecode,
    WrongPacketToEncode,
    PropertyNotFound,
    ShortData,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match *self {
            Error::Utf8Error => write!(f, "Error encountered during UTF8 decoding!"),
            Error::InsufficientBufferSize => write!(f, "Buffer size is not sufficient for packet!"),
            Error::VariableByteIntegerError => write!(f, "Error encountered during variable byte integer decoding / encoding!"),
            Error::IdNotFound => write!(f, "Packet identifier not found!"),
            Error::EncodingError => write!(f, "Error encountered during packet encoding!"),
            Error::DecodingError => write!(f, "Error encountered during packet decoding!"),
            Error::PacketTypeMismatch => write!(f, "Packet type not matched during decoding (Received different packet type than encode type)!"),
            Error::WrongPacketToDecode => write!(f, "Not able to decode packet, this packet is used just for sending to broker, not receiving by client!"),
            Error::WrongPacketToEncode => write!(f, "Not able to encode packet, this packet is used only from server to client not the opposite way!"),
            Error::PropertyNotFound => write!(f, "Property with ID not found!"),
            Error::ShortData => write!(f, "The data included to decode the packet is to short"),
        }
    }
}
