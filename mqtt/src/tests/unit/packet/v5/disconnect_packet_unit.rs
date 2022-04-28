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

use heapless::Vec;

use crate::packet::v5::disconnect_packet::DisconnectPacket;
use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::packet_type::PacketType;
use crate::packet::v5::property::Property;
use crate::utils::buffer_reader::BuffReader;

#[test]
fn test_encode() {
    let mut buffer: [u8; 10] = [0; 10];
    let mut packet = DisconnectPacket::<1>::new();
    let prop: Property = Property::SessionExpiryInterval(512);
    let mut props = Vec::<Property, 1>::new();
    props.push(prop);
    packet.property_len = packet.add_properties(&props);
    let res = packet.encode(&mut buffer, 100);
    assert!(res.is_ok());
    assert_eq!(
        buffer[0..res.unwrap()],
        [0xE0, 0x07, 0x00, 0x05, 0x11, 0x00, 0x00, 0x02, 0x00]
    )
}

#[test]
fn test_decode() {
    let buffer: [u8; 10] = [0xE0, 0x07, 0x00, 0x05, 0x11, 0x00, 0x00, 0x04, 0x00, 0x00];
    let mut packet = DisconnectPacket::<1>::new();
    let res = packet.decode(&mut BuffReader::new(&buffer, 10));
    assert!(res.is_ok());
    assert_eq!(packet.fixed_header, PacketType::Disconnect.into());
    assert_eq!(packet.remain_len, 7);
    assert_eq!(packet.disconnect_reason, 0x00);
    assert_eq!(packet.property_len, 5);
    let prop = packet.properties.get(0);
    assert!(prop.is_some());
    assert_eq!(<&Property as Into<u8>>::into(prop.unwrap()), 0x11);
    if let Property::SessionExpiryInterval(u) = *prop.unwrap() {
        assert_eq!(u, 1024);
    }
}
