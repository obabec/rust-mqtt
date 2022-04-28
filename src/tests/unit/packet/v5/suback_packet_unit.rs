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
use crate::packet::v5::property::Property;
use crate::packet::v5::suback_packet::SubackPacket;
use crate::utils::buffer_reader::BuffReader;

#[test]
fn test_decode() {
    let buffer: [u8; 23] = [
        0x90, 0x15, 0xCC, 0x08, 0x0F, 0x1F, 0x00, 0x0C, 0x72, 0x65, 0x61, 0x73, 0x6f, 0x6e, 0x53,
        0x74, 0x72, 0x69, 0x6e, 0x67, 0x12, 0x34, 0x56,
    ];
    let mut packet = SubackPacket::<3, 1>::new();
    let res = packet.decode(&mut BuffReader::new(&buffer, 23));
    assert!(res.is_ok());
    assert_eq!(packet.fixed_header, PacketType::Suback.into());
    assert_eq!(packet.remain_len, 21);
    assert_eq!(packet.packet_identifier, 52232);
    assert_eq!(packet.property_len, 15);
    let prop = packet.properties.get(0);
    assert!(prop.is_some());
    assert_eq!(<&Property as Into<u8>>::into(prop.unwrap()), 0x1F);
    if let Property::ReasonString(u) = (*prop.unwrap()).clone() {
        assert_eq!(u.len, 12);
        assert_eq!(u.string, "reasonString");
    }
    assert_eq!(packet.reason_codes.len(), 3);
    let res1 = packet.reason_codes.get(0);
    assert!(res1.is_some());
    if let Some(r) = res1 {
        assert_eq!(*r, 0x12);
    }
    let res2 = packet.reason_codes.get(1);
    assert!(res2.is_some());
    if let Some(r) = res2 {
        assert_eq!(*r, 0x34);
    }
    let res3 = packet.reason_codes.get(2);
    assert!(res3.is_some());
    if let Some(r) = res3 {
        assert_eq!(*r, 0x56);
    }
}
