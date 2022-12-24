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

// x x x x - - - -

#[derive(PartialEq)]
pub enum PacketType {
    Reserved,
    Connect,
    Connack,
    Publish,
    Puback,
    Pubrec,
    Pubrel,
    Pubcomp,
    Subscribe,
    Suback,
    Unsubscribe,
    Unsuback,
    Pingreq,
    Pingresp,
    Disconnect,
    Auth,
}

impl From<u8> for PacketType {
    fn from(orig: u8) -> Self {
        let packet_type: u8 = orig & 0xF0;
        match packet_type {
            0x10 => PacketType::Connect,
            0x20 => PacketType::Connack,
            0x00 => PacketType::Reserved,
            0x30 => PacketType::Publish,
            0x40 => PacketType::Puback,
            0x50 => PacketType::Pubrec,
            0x60 => PacketType::Pubrel,
            0x70 => PacketType::Pubcomp,
            0x80 => PacketType::Subscribe,
            0x90 => PacketType::Suback,
            0xA0 => PacketType::Unsubscribe,
            0xB0 => PacketType::Unsuback,
            0xC0 => PacketType::Pingreq,
            0xD0 => PacketType::Pingresp,
            0xE0 => PacketType::Disconnect,
            0xF0 => PacketType::Auth,
            _ => PacketType::Reserved,
        }
    }
}

impl From<PacketType> for u8 {
    fn from(value: PacketType) -> Self {
        match value {
            PacketType::Connect => 0x10,
            PacketType::Connack => 0x20,
            PacketType::Publish => 0x30,
            PacketType::Puback => 0x40,
            PacketType::Pubrec => 0x50,
            PacketType::Pubrel => 0x62,
            PacketType::Pubcomp => 0x70,
            PacketType::Subscribe => 0x82,
            PacketType::Suback => 0x90,
            PacketType::Unsubscribe => 0xA2,
            PacketType::Unsuback => 0xB0,
            PacketType::Pingreq => 0xC0,
            PacketType::Pingresp => 0xD0,
            PacketType::Disconnect => 0xE0,
            PacketType::Auth => 0xF0,
            PacketType::Reserved => 0x00,
        }
    }
}
