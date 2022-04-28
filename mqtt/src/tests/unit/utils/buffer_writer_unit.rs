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
use tokio_test::{assert_err, assert_ok};

use crate::encoding::variable_byte_integer::VariableByteInteger;
use crate::packet::v5::property::Property;
use crate::utils::buffer_writer::BuffWriter;
use crate::utils::types::{BinaryData, BufferError, EncodedString, StringPair, TopicFilter};

#[test]
fn buffer_write_ref() {
    static BUFFER: [u8; 5] = [0x82, 0x82, 0x03, 0x85, 0x84];
    let mut res_buffer: [u8; 5] = [0; 5];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.insert_ref(5, &BUFFER);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 5);
    assert_eq!(BUFFER, res_buffer);
}

#[test]
fn buffer_write_ref_oob() {
    static BUFFER: [u8; 5] = [0x82, 0x82, 0x03, 0x85, 0x84];
    let mut res_buffer: [u8; 4] = [0; 4];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 4);
    let test_number = writer.insert_ref(5, &BUFFER);
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
    assert_eq!(res_buffer, [0; 4])
}

#[test]
fn buffer_write_u8() {
    let mut res_buffer: [u8; 1] = [0; 1];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 1);
    let test_write = writer.write_u8(0xFA);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 1);
    assert_eq!(res_buffer, [0xFA]);
}

#[test]
fn buffer_write_u8_oob() {
    let mut res_buffer: [u8; 0] = [];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 0);
    let test_number = writer.write_u8(0xFA);
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn buffer_write_u16() {
    let mut res_buffer: [u8; 2] = [0; 2];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 2);
    let test_write = writer.write_u16(0xFAED);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 2);
    assert_eq!(res_buffer, [0xFA, 0xED]);
}

#[test]
fn buffer_write_u16_oob() {
    let mut res_buffer: [u8; 1] = [0; 1];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 1);
    let test_number = writer.write_u16(0xFAED);
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn buffer_write_u32() {
    let mut res_buffer: [u8; 4] = [0; 4];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 4);
    let test_write = writer.write_u32(0xFAEDCC09);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 4);
    assert_eq!(res_buffer, [0xFA, 0xED, 0xCC, 0x09]);
}

#[test]
fn buffer_write_u32_oob() {
    let mut res_buffer: [u8; 3] = [0; 3];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 3);
    let test_number = writer.write_u32(0xFAEDCC08);
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn buffer_write_string() {
    let mut res_buffer: [u8; 6] = [0; 6];
    let mut string = EncodedString::new();
    string.string = "😎";
    string.len = 4;
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 6);
    let test_write = writer.write_string_ref(&string);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 6);
    assert_eq!(res_buffer, [0x00, 0x04, 0xF0, 0x9F, 0x98, 0x8E]);
}

#[test]
fn buffer_write_string_oob() {
    let mut res_buffer: [u8; 5] = [0; 5];
    let mut string = EncodedString::new();
    string.string = "😎";
    string.len = 4;
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.write_string_ref(&string);
    assert!(test_write.is_err());
    assert_eq!(test_write.unwrap_err(), BufferError::InsufficientBufferSize);
}

#[test]
fn buffer_write_bin() {
    let mut res_buffer: [u8; 6] = [0; 6];
    let mut bin = BinaryData::new();
    bin.bin = &[0xAB, 0xEF, 0x88, 0x43];
    bin.len = 4;
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 6);
    let test_write = writer.write_binary_ref(&bin);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 6);
    assert_eq!(res_buffer, [0x00, 0x04, 0xAB, 0xEF, 0x88, 0x43]);
}

#[test]
fn buffer_write_bin_oob() {
    let mut res_buffer: [u8; 6] = [0; 6];
    let mut bin = BinaryData::new();
    bin.bin = &[0xAB, 0xEF, 0x88, 0x43];
    bin.len = 4;
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.write_binary_ref(&bin);
    assert!(test_write.is_err());
    assert_eq!(test_write.unwrap_err(), BufferError::InsufficientBufferSize);
}

#[test]
fn buffer_write_string_pair() {
    let mut res_buffer: [u8; 12] = [0; 12];
    let mut name = EncodedString::new();
    name.string = "Name";
    name.len = 4;

    let mut value = EncodedString::new();
    value.string = "😎";
    value.len = 4;
    let mut pair = StringPair::new();
    pair.name = name;
    pair.value = value;
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 12);
    let test_write = writer.write_string_pair_ref(&pair);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 12);
    assert_eq!(
        res_buffer,
        [0x00, 0x04, 0x4e, 0x61, 0x6d, 0x65, 0x00, 0x04, 0xF0, 0x9F, 0x98, 0x8E]
    );
}

#[test]
fn buffer_write_string_pair_oob() {
    let mut res_buffer: [u8; 12] = [0; 12];
    let mut name = EncodedString::new();
    name.string = "Name";
    name.len = 4;

    let mut value = EncodedString::new();
    value.string = "😎";
    value.len = 4;
    let mut pair = StringPair::new();
    pair.name = name;
    pair.value = value;
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 10);
    let test_write = writer.write_string_pair_ref(&pair);
    assert!(test_write.is_err());
    assert_eq!(test_write.unwrap_err(), BufferError::InsufficientBufferSize)
}

#[test]
fn buffer_write_var_byte() {
    let mut res_buffer: [u8; 2] = [0; 2];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 2);
    let test_write = writer.write_variable_byte_int(512);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 2);
    assert_eq!(res_buffer, [0x80, 0x04]);
}

#[test]
fn buffer_write_var_byte_oob() {
    let mut res_buffer: [u8; 2] = [0; 2];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 2);
    let test_number = writer.write_variable_byte_int(453123);
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

/*#[test]
fn buffer_write_property() {
    let mut res_buffer: [u8; 7] = [0; 7];
    let mut topic = EncodedString::new();
    topic.string = "Name";
    topic.len = 4;
    let prop = Property::ResponseTopic(topic);
    let mut writer: BuffWriter = BuffWriter::new(& mut res_buffer, 7);
    let test_write = writer.write_property(&prop);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 7);
    assert_eq!(res_buffer, [0x08, 0x00, 0x04, 0x4e, 0x61, 0x6d, 0x65]);
}

#[test]
fn buffer_write_property_oob() {
    let mut res_buffer: [u8; 7] = [0; 7];
    let mut topic = EncodedString::new();
    topic.string = "Name";
    topic.len = 4;
    let prop = Property::ResponseTopic(topic);
    let mut writer: BuffWriter = BuffWriter::new(& mut res_buffer, 4);
    let test_write = writer.write_property(&prop);
    assert!(test_write.is_err());
    assert_eq!(test_write.unwrap_err(), BufferError::InsufficientBufferSize);
}*/

#[test]
fn buffer_write_properties() {
    let mut res_buffer: [u8; 13] = [0; 13];
    let mut topic = EncodedString::new();
    topic.string = "Name";
    topic.len = 4;
    let prop = Property::ResponseTopic(topic);
    let mut corr = BinaryData::new();
    corr.bin = &[0x12, 0x34, 0x56];
    corr.len = 3;
    let prop2 = Property::CorrelationData(corr);
    let mut properties = Vec::<Property, 2>::new();
    properties.push(prop);
    properties.push(prop2);
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 13);
    let test_write = writer.write_properties(&properties);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 13);
    assert_eq!(
        res_buffer,
        [0x08, 0x00, 0x04, 0x4e, 0x61, 0x6d, 0x65, 0x09, 0x00, 0x03, 0x12, 0x34, 0x56]
    );
}

#[test]
fn buffer_write_properties_oob() {
    let mut res_buffer: [u8; 10] = [0; 10];
    let mut topic = EncodedString::new();
    topic.string = "Name";
    topic.len = 4;
    let prop = Property::ResponseTopic(topic);
    let mut corr = BinaryData::new();
    corr.bin = &[0x12, 0x34, 0x56];
    corr.len = 3;
    let prop2 = Property::CorrelationData(corr);
    let mut properties = Vec::<Property, 2>::new();
    properties.push(prop);
    properties.push(prop2);
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 10);
    let test_write = writer.write_properties(&properties);
    assert!(test_write.is_err());
    assert_eq!(test_write.unwrap_err(), BufferError::InsufficientBufferSize);
}

#[test]
fn buffer_write_filters() {
    let mut res_buffer: [u8; 15] = [0; 15];
    static STR1: &str = "test";
    static STR2: &str = "topic";
    let mut topic = EncodedString::new();
    topic.string = STR1;
    topic.len = 4;

    let mut topic2 = EncodedString::new();
    topic2.string = STR2;
    topic2.len = 5;

    let mut filter1 = TopicFilter::new();
    filter1.filter = topic;
    filter1.sub_options = 0xAE;

    let mut filter2 = TopicFilter::new();
    filter2.filter = topic2;
    filter2.sub_options = 0x22;

    let mut filters = Vec::<TopicFilter, 2>::new();
    filters.push(filter1);
    filters.push(filter2);
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 15);
    let test_write = writer.write_topic_filters_ref(true, 2, &filters);
    assert!(test_write.is_ok());
    assert_eq!(writer.position, 15);
    assert_eq!(
        res_buffer,
        [
            0x00, 0x04, 0x74, 0x65, 0x73, 0x74, 0xAE, 0x00, 0x05, 0x74, 0x6f, 0x70, 0x69, 0x63,
            0x22
        ]
    );
}

#[test]
fn buffer_write_filters_oob() {
    let mut res_buffer: [u8; 15] = [0; 15];
    static STR1: &str = "test";
    static STR2: &str = "topic";
    let mut topic = EncodedString::new();
    topic.string = STR1;
    topic.len = 4;

    let mut topic2 = EncodedString::new();
    topic2.string = STR2;
    topic2.len = 5;

    let mut filter1 = TopicFilter::new();
    filter1.filter = topic;
    filter1.sub_options = 0xAE;

    let mut filter2 = TopicFilter::new();
    filter2.filter = topic2;
    filter2.sub_options = 0x22;

    let mut filters = Vec::<TopicFilter, 2>::new();
    filters.push(filter1);
    filters.push(filter2);
    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.write_topic_filters_ref(true, 2, &filters);
    assert!(test_write.is_err());
    assert_eq!(test_write.unwrap_err(), BufferError::InsufficientBufferSize)
}

#[test]
fn buffer_get_rem_len_one() {
    static BUFFER: [u8; 5] = [0x82, 0x02, 0x03, 0x85, 0x84];
    static REF: VariableByteInteger = [0x02, 0x00, 0x00, 0x00];
    let mut res_buffer: [u8; 5] = [0; 5];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.insert_ref(5, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_two() {
    static BUFFER: [u8; 5] = [0x82, 0x82, 0x03, 0x85, 0x84];
    static REF: VariableByteInteger = [0x82, 0x03, 0x00, 0x00];
    let mut res_buffer: [u8; 5] = [0; 5];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.insert_ref(5, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_three() {
    static BUFFER: [u8; 5] = [0x82, 0x82, 0x83, 0x05, 0x84];
    static REF: VariableByteInteger = [0x82, 0x83, 0x05, 0x00];
    let mut res_buffer: [u8; 5] = [0; 5];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.insert_ref(5, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_all() {
    static BUFFER: [u8; 5] = [0x82, 0x82, 0x83, 0x85, 0x04];
    static REF: VariableByteInteger = [0x82, 0x83, 0x85, 0x04];
    let mut res_buffer: [u8; 5] = [0; 5];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 5);
    let test_write = writer.insert_ref(5, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_over() {
    static BUFFER: [u8; 6] = [0x82, 0x82, 0x83, 0x85, 0x84, 0x34];
    static REF: VariableByteInteger = [0x82, 0x83, 0x85, 0x84];
    let mut res_buffer: [u8; 6] = [0; 6];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 6);
    let test_write = writer.insert_ref(6, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_zero_end() {
    static BUFFER: [u8; 6] = [0x82, 0x82, 0x83, 0x85, 0x04, 0x34];
    static REF: VariableByteInteger = [0x82, 0x83, 0x85, 0x04];
    let mut res_buffer: [u8; 6] = [0; 6];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 6);
    let test_write = writer.insert_ref(6, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_zero() {
    static BUFFER: [u8; 6] = [0x82, 0x00, 0x83, 0x85, 0x04, 0x34];
    static REF: VariableByteInteger = [0x00, 0x00, 0x00, 0x00];
    let mut res_buffer: [u8; 6] = [0; 6];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 6);
    let test_write = writer.insert_ref(6, &BUFFER);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_ok!(rm_len);
    assert_eq!(rm_len.unwrap(), REF);
}

#[test]
fn buffer_get_rem_len_cont() {
    static BUFFER: [u8; 6] = [0x82, 0x00, 0x83, 0x85, 0x04, 0x34];
    static REF: VariableByteInteger = [0x00, 0x00, 0x00, 0x00];
    let mut res_buffer: [u8; 6] = [0; 6];

    let mut writer: BuffWriter = BuffWriter::new(&mut res_buffer, 6);
    let test_write = writer.insert_ref(2, &[0x82, 0x81]);
    let rm_len = writer.get_rem_len();
    assert_ok!(test_write);
    assert_err!(rm_len);
    writer.insert_ref(2, &[0x82, 0x01]);
    let rm_len_sec = writer.get_rem_len();
    assert_ok!(rm_len_sec);
    assert_eq!(rm_len_sec.unwrap(), [0x81, 0x82, 0x01, 0x00]);
}
