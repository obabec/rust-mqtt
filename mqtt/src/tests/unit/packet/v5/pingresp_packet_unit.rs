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

use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::packet_type::PacketType;
use crate::packet::v5::pingresp_packet::PingrespPacket;
use crate::utils::buffer_reader::BuffReader;

#[test]
fn test_encode() {
    let mut buffer: [u8; 3] = [0x00, 0x98, 0x45];
    let mut packet = PingrespPacket::new();
    packet.fixed_header = PacketType::Pingresp.into();
    packet.remain_len = 0;
    let res = packet.encode(&mut buffer, 3);
    assert!(res.is_ok());
    assert_eq!(buffer, [0xD0, 0x00, 0x45])
}

#[test]
fn test_decode() {
    let buffer: [u8; 3] = [0xD0, 0x00, 0x51];
    let mut packet = PingrespPacket::new();
    let res = packet.decode(&mut BuffReader::new(&buffer, 3));
    assert!(res.is_ok());
    assert_eq!(packet.fixed_header, PacketType::Pingresp.into());
    assert_eq!(packet.remain_len, 0);
}
