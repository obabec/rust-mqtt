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

use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::packet_type::PacketType;
use crate::packet::v5::property::Property;
use crate::packet::v5::pubrel_packet::PubrelPacket;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::types::{EncodedString, StringPair};

#[test]
fn test_encode() {
    let mut buffer: [u8; 21] = [0; 21];
    let mut packet = PubrelPacket::<1>::new();
    packet.fixed_header = PacketType::Pubrel.into();
    packet.packet_identifier = 12345;
    packet.reason_code = 0x86;
    let mut name = EncodedString::new();
    name.string = "haha";
    name.len = 4;
    let mut val = EncodedString::new();
    val.string = "hehe89";
    val.len = 6;
    let mut pair = StringPair::new();
    pair.name = name;
    pair.value = val;
    let mut props = Vec::<Property, 1>::new();
    props.push(Property::UserProperty(pair));
    packet.property_len = packet.add_properties(&props);
    let res = packet.encode(&mut buffer, 21);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 21);
    assert_eq!(
        buffer,
        [
            0x62, 0x13, 0x30, 0x39, 0x86, 0x0F, 0x26, 0x00, 0x04, 0x68, 0x61, 0x68, 0x61, 0x00,
            0x06, 0x68, 0x65, 0x68, 0x65, 0x38, 0x39
        ]
    )
}

#[test]
fn test_decode() {
    let buffer: [u8; 21] = [
        0x62, 0x13, 0x30, 0x39, 0x86, 0x0F, 0x26, 0x00, 0x04, 0x68, 0x61, 0x68, 0x61, 0x00, 0x06,
        0x68, 0x65, 0x68, 0x65, 0x38, 0x39,
    ];
    let mut packet = PubrelPacket::<1>::new();
    let res = packet.decode(&mut BuffReader::new(&buffer, 21));
    assert!(res.is_ok());
    assert_eq!(packet.fixed_header, PacketType::Pubrel.into());
    assert_eq!(packet.remain_len, 19);
    assert_eq!(packet.packet_identifier, 12345);
    assert_eq!(packet.reason_code, 0x86);
    assert_eq!(packet.property_len, 15);
    let prop = packet.properties.get(0);
    assert!(prop.is_some());
    assert_eq!(<&Property as Into<u8>>::into(prop.unwrap()), 0x26);
    if let Property::UserProperty(u) = (*prop.unwrap()).clone() {
        assert_eq!(u.name.len, 4);
        assert_eq!(u.name.string, "haha");
        assert_eq!(u.value.len, 6);
        assert_eq!(u.value.string, "hehe89");
    }
}
