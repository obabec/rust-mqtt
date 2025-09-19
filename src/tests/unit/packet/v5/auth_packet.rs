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

use crate::encoding::EncodedString;
use crate::interface::Property;
use crate::io::BuffReader;
use crate::packet::v5::auth_packet::AuthPacket;
use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::packet_type::PacketType;

#[test]
fn test_encode() {
    let mut buffer: [u8; 20] = [0; 20];
    let mut auth = AuthPacket::<1>::new();

    auth.auth_reason = 0x18;

    let mut auth_method = EncodedString::new();
    auth_method.string = "SCRAM-SHA-1";
    auth_method.len = 11;
    let mut props = Vec::<Property, 1>::new();
    props.push(Property::AuthenticationMethod(auth_method));
    auth.property_len = auth.add_properties(&props);

    let res = auth.encode(&mut buffer, 20);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 18);
    assert_eq!(
        buffer[0..18],
        [
            0xF0, 0x10, 0x18, 0x0E, 0x15, 0x00, 0x0B, 0x53, 0x43, 0x52, 0x41, 0x4D, 0x2D, 0x53,
            0x48, 0x41, 0x2D, 0x31
        ]
    )
}

#[test]
fn test_decode() {
    let buffer: [u8; 11] = [
        0xF0, 0x09, 0x18, 0x07, 0x15, 0x00, 0x04, 0x53, 0x41, 0x53, 0x4C,
    ];
    let mut auth = AuthPacket::<1>::new();
    let res = auth.decode(&mut BuffReader::new(&buffer, 11));
    assert!(res.is_ok());
    assert_eq!(auth.fixed_header, PacketType::Auth.into());
    assert_eq!(auth.remain_len, 9);
    assert_eq!(auth.auth_reason, 0x18);
    assert_eq!(auth.property_len, 7);
    let prop = auth.properties.get(0);
    assert!(prop.is_some());
    assert_eq!(<&Property as Into<u8>>::into(prop.unwrap()), 0x15);
    if let Property::AuthenticationMethod(method) = (*prop.unwrap()).clone() {
        assert_eq!(method.string, "SASL");
        assert_eq!(method.len, 4);
    }
}
