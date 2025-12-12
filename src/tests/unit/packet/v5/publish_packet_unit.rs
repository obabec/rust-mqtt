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
use crate::interface::QualityOfService;
use crate::io::BuffReader;
use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::packet_type::PacketType;
use crate::packet::v5::publish_packet::PublishPacket;

#[test]
fn test_encode() {
    let mut buffer: [u8; 29] = [0; 29];
    let mut packet = PublishPacket::<2>::new();
    packet.fixed_header = PacketType::Publish.into();
    packet.add_qos(QualityOfService::QoS1);
    let mut topic = EncodedString::new();
    topic.string = "test";
    topic.len = 4;
    packet.topic_name = topic;
    packet.packet_identifier = 23432;
    let mut props = Vec::<Property, 2>::new();
    props.push(Property::PayloadFormatIndicator(0x01));
    props.push(Property::MessageExpiryInterval(45678));
    packet.property_len = packet.add_properties(&props);
    static MESSAGE: [u8; 11] = [
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ];
    packet.add_message(&MESSAGE);
    let res = packet.encode(&mut buffer, 100);
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 29);
    assert_eq!(
        buffer,
        [
            0x32, 0x1B, 0x00, 0x04, 0x74, 0x65, 0x73, 0x74, 0x5B, 0x88, 0x07, 0x01, 0x01, 0x02,
            0x00, 0x00, 0xB2, 0x6E, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c,
            0x64
        ]
    )
}

#[test]
fn test_decode() {
    let buffer: [u8; 29] = [
        0x32, 0x1B, 0x00, 0x04, 0x74, 0x65, 0x73, 0x74, 0x5B, 0x88, 0x07, 0x01, 0x01, 0x02, 0x00,
        0x00, 0xB2, 0x6E, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ];
    let mut packet = PublishPacket::<2>::new();
    let res = packet.decode(&mut BuffReader::new(&buffer, 29));
    assert!(res.is_ok());
    assert_eq!(packet.fixed_header, 0x32);
    assert_eq!(packet.topic_name.len, 4);
    assert_eq!(packet.topic_name.string, "test");
    assert_eq!(packet.packet_identifier, 23432);
    assert_eq!(packet.property_len, 7);
    let prop = packet.properties.get(0);
    assert!(prop.is_some());
    assert_eq!(<&Property as Into<u8>>::into(prop.unwrap()), 0x01);
    if let Property::PayloadFormatIndicator(u) = (*prop.unwrap()).clone() {
        assert_eq!(u, 0x01);
    }
    let prop2 = packet.properties.get(1);
    assert!(prop2.is_some());
    assert_eq!(<&Property as Into<u8>>::into(prop2.unwrap()), 0x02);
    if let Property::MessageExpiryInterval(u) = (*prop2.unwrap()).clone() {
        assert_eq!(u, 45678);
    }
    if let Some(message) = packet.message {
        assert_eq!(
            *message,
            [0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64]
        );
    }
}

#[test]
fn test_decode_with_correlation_data() {
    // mosquitto_pub -h test.mosquitto.org -t "senbax/bugfix" -m "payloadpayload" --property PUBLISH correlation-data correlation
    let buffer: [u8; 46] = [
        0x30, 0x2c, 0x00, 0x0d, 0x73, 0x65, 0x6e, 0x62, 0x61, 0x78, 0x2f, 0x62, 0x75, 0x67, 0x66,
        0x69, 0x78, 0x0e, 0x09, 0x00, 0x0b, 0x63, 0x6f, 0x72, 0x72, 0x65, 0x6c, 0x61, 0x74, 0x69,
        0x6f, 0x6e, 0x70, 0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64, 0x70, 0x61, 0x79, 0x6c, 0x6f, 0x61,
        0x64,
    ];
    let mut packet = PublishPacket::<2>::new();
    let res = packet.decode(&mut BuffReader::new(&buffer, 46));
    assert!(res.is_ok());
    assert_eq!(packet.topic_name.len, 13);
    assert_eq!(packet.topic_name.string, "senbax/bugfix");
    assert_eq!(packet.property_len, 14);

    assert!(packet.properties.len() == 1);

    let prop = packet.properties.get(0);
    assert!(prop.is_some());
    if let Some(message) = packet.message {
        assert_eq!(
            *message,
            [0x70, 0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64, 0x70, 0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64]
        );
    }
}
