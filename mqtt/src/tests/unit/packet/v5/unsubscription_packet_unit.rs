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
use crate::packet::v5::unsubscription_packet::UnsubscriptionPacket;
use crate::utils::types::{EncodedString, StringPair};

#[test]
fn test_encode() {
    let mut buffer: [u8; 40] = [0; 40];
    let mut packet = UnsubscriptionPacket::<2, 1>::new();
    packet.fixed_header = PacketType::Unsubscribe.into();
    packet.packet_identifier = 5432;
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
    packet.add_new_filter("test/topic");
    packet.add_new_filter("hehe/#");
    let res = packet.encode(&mut buffer, 40);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 40);
    assert_eq!(
        buffer,
        [
            0xA2, 0x26, 0x15, 0x38, 0x0F, 0x26, 0x00, 0x04, 0x68, 0x61, 0x68, 0x61, 0x00, 0x06,
            0x68, 0x65, 0x68, 0x65, 0x38, 0x39, 0x00, 0x0A, 0x74, 0x65, 0x73, 0x74, 0x2F, 0x74,
            0x6F, 0x70, 0x69, 0x63, 0x00, 0x06, 0x68, 0x65, 0x68, 0x65, 0x2F, 0x23
        ]
    );
}
