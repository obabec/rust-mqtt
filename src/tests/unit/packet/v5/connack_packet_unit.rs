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

use crate::packet::v5::connack_packet::ConnackPacket;
use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::property::Property;
use crate::packet::v5::reason_codes::ReasonCode;
use crate::utils::buffer_reader::BuffReader;

#[test]
fn test_encode() {
    let mut buffer: [u8; 100] = [0; 100];
    let mut connack = ConnackPacket::<2>::new();
    connack.property_len = 3;
    let prop = Property::ReceiveMaximum(21);
    connack.properties.push(prop);
    connack.connect_reason_code = ReasonCode::ServerMoved.into();
    connack.ack_flags = 0x45;

    let res = connack.encode(&mut buffer, 100);
    assert!(res.is_ok());
    assert_eq!(
        buffer[0..res.unwrap()],
        [
            0x20,
            0x06,
            0x45,
            ReasonCode::ServerMoved.into(),
            0x03,
            0x21,
            0x00,
            0x15
        ]
    )
}

#[test]
fn test_decode() {
    let mut buffer: [u8; 8] = [
        0x20,
        0x06,
        0x45,
        ReasonCode::ServerMoved.into(),
        0x03,
        0x21,
        0x00,
        0x15,
    ];
    let mut connack_res = ConnackPacket::<2>::new();
    let res = connack_res.decode(&mut BuffReader::new(&buffer, 8));

    assert!(res.is_ok());
    assert_eq!(connack_res.property_len, 3);
    assert_eq!(connack_res.ack_flags, 0x45);
    assert_eq!(
        connack_res.connect_reason_code,
        ReasonCode::ServerMoved.into()
    );
    assert_eq!(connack_res.property_len, 3);
    let prop = connack_res.properties.get(0).unwrap();
    assert_eq!(<&Property as Into<u8>>::into(prop), 0x21);
    if let Property::ReceiveMaximum(u) = *prop {
        assert_eq!(u, 21);
    }
}
