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

use crate::utils::buffer_reader::BuffReader;
use crate::utils::types::BufferError;

#[test]
fn buffer_read_variable_byte() {
    static BUFFER: [u8; 5] = [0x82, 0x82, 0x03, 0x85, 0x84];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 5);
    let test_number = reader.read_variable_byte_int();
    assert!(test_number.is_ok());
    assert_eq!(reader.position, 3);
    assert_eq!(test_number.unwrap(), 49410);
}

#[test]
fn buffer_read_invalid_size() {
    static BUFFER: [u8; 2] = [0x82, 0x82];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 2);
    let test_number = reader.read_variable_byte_int();
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn test_smaller_var_int() {
    static BUFFER: [u8; 2] = [0x82, 0x02];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 2);
    let test_number = reader.read_variable_byte_int();
    assert!(test_number.is_ok());
    assert_eq!(reader.position, 2);
    assert_eq!(test_number.unwrap(), 258);
}

#[test]
fn test_complete_var_int() {
    static BUFFER: [u8; 4] = [0x81, 0x81, 0x81, 0x01];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 4);
    let test_number = reader.read_variable_byte_int();
    assert!(test_number.is_ok());
    assert_eq!(reader.position, 4);
    assert_eq!(test_number.unwrap(), 2113665);
}

#[test]
fn test_var_empty_buffer() {
    static BUFFER: [u8; 0] = [];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 0);
    let test_number = reader.read_variable_byte_int();
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn test_read_u32() {
    static BUFFER: [u8; 4] = [0x00, 0x02, 0x5E, 0xC1];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 4);
    let test_number = reader.read_u32();
    assert!(test_number.is_ok());
    assert_eq!(test_number.unwrap(), 155329);
}

#[test]
fn test_read_u32_oob() {
    static BUFFER: [u8; 3] = [0x00, 0x02, 0x5E];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 3);
    let test_number = reader.read_u32();
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn test_read_u16() {
    static BUFFER: [u8; 2] = [0x48, 0x5F];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 2);
    let test_number = reader.read_u16();
    assert!(test_number.is_ok());
    assert_eq!(test_number.unwrap(), 18527);
}

#[test]
fn test_read_u16_oob() {
    static BUFFER: [u8; 1] = [0x5E];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 1);
    let test_number = reader.read_u16();
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn test_read_u8() {
    static BUFFER: [u8; 1] = [0xFD];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 1);
    let test_number = reader.read_u8();
    assert!(test_number.is_ok());
    assert_eq!(test_number.unwrap(), 253);
}

#[test]
fn test_read_u8_oob() {
    static BUFFER: [u8; 0] = [];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 0);
    let test_number = reader.read_u8();
    assert!(test_number.is_err());
    assert_eq!(
        test_number.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn test_read_string() {
    static BUFFER: [u8; 6] = [0x00, 0x04, 0xF0, 0x9F, 0x92, 0x96];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 6);
    let test_string = reader.read_string();
    assert!(test_string.is_ok());
    let unw = test_string.unwrap();
    assert_eq!(unw.string, "💖");
    assert_eq!(unw.len, 4);
}

#[test]
fn test_read_string_utf8_wrong() {
    static BUFFER: [u8; 5] = [0x00, 0x03, 0xF0, 0x9F, 0x92];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 5);
    let test_string = reader.read_string();
    assert!(test_string.is_err());
    assert_eq!(test_string.unwrap_err(), BufferError::Utf8Error);
}

#[test]
fn test_read_string_oob() {
    static BUFFER: [u8; 5] = [0x00, 0x04, 0xF0, 0x9F, 0x92];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 5);
    let test_string = reader.read_string();
    assert!(test_string.is_err());
    assert_eq!(
        test_string.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}

#[test]
fn test_read_binary() {
    static BUFFER: [u8; 6] = [0x00, 0x04, 0xFF, 0xEE, 0xDD, 0xCC];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 6);
    let test_bin = reader.read_binary();
    assert!(test_bin.is_ok());
    let unw = test_bin.unwrap();
    assert_eq!(unw.bin, [0xFF, 0xEE, 0xDD, 0xCC]);
    assert_eq!(unw.len, 4);
}

#[test]
fn test_read_binary_oob() {
    static BUFFER: [u8; 5] = [0x00, 0x04, 0xFF, 0xEE, 0xDD];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 5);
    let test_bin = reader.read_binary();
    assert!(test_bin.is_err());
    assert_eq!(test_bin.unwrap_err(), BufferError::InsufficientBufferSize);
}

#[test]
fn test_read_string_pair() {
    static BUFFER: [u8; 11] = [
        0x00, 0x04, 0xF0, 0x9F, 0x98, 0x8E, 0x00, 0x03, 0xE2, 0x93, 0x87,
    ];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 11);
    let string_pair = reader.read_string_pair();
    assert!(string_pair.is_ok());
    let unw = string_pair.unwrap();
    assert_eq!(unw.name.string, "😎");
    assert_eq!(unw.name.len, 4);
    assert_eq!(unw.value.string, "Ⓡ");
    assert_eq!(unw.value.len, 3);
}

#[test]
fn test_read_string_pair_wrong_utf8() {
    static BUFFER: [u8; 11] = [
        0x00, 0x03, 0xF0, 0x9F, 0x92, 0x00, 0x04, 0xF0, 0x9F, 0x98, 0x8E,
    ];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 11);
    let string_pair = reader.read_string_pair();
    assert!(string_pair.is_err());
    assert_eq!(string_pair.unwrap_err(), BufferError::Utf8Error)
}

#[test]
fn test_read_string_pair_oob() {
    static BUFFER: [u8; 11] = [
        0x00, 0x04, 0xF0, 0x9F, 0x98, 0x8E, 0x00, 0x04, 0xE2, 0x93, 0x87,
    ];
    let mut reader: BuffReader = BuffReader::new(&BUFFER, 11);
    let string_pair = reader.read_string_pair();
    assert!(string_pair.is_err());
    assert_eq!(
        string_pair.unwrap_err(),
        BufferError::InsufficientBufferSize
    );
}
