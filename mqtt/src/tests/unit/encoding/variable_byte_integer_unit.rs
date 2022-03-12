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

use crate::encoding::variable_byte_integer::{
    VariableByteInteger, VariableByteIntegerDecoder, VariableByteIntegerEncoder,
};
use crate::utils::types::BufferError;

#[test]
fn test_decode() {
    static BUFFER: VariableByteInteger = [0x81, 0x81, 0x81, 0x01];

    let decoded = VariableByteIntegerDecoder::decode(BUFFER);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), 2113665);
}

#[test]
fn test_decode_small() {
    static BUFFER: VariableByteInteger = [0x81, 0x81, 0x01, 0x85];

    let decoded = VariableByteIntegerDecoder::decode(BUFFER);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), 16_513);
}

#[test]
fn test_encode() {
    let encoded = VariableByteIntegerEncoder::encode(211_366_5);
    assert!(encoded.is_ok());
    let res = encoded.unwrap();
    assert_eq!(res, [0x81, 0x81, 0x81, 0x01]);
    assert_eq!(VariableByteIntegerEncoder::len(res), 4);
}

#[test]
fn test_encode_small() {
    let encoded = VariableByteIntegerEncoder::encode(16_513);
    assert!(encoded.is_ok());
    let res = encoded.unwrap();
    assert_eq!(res, [0x81, 0x81, 0x01, 0x00]);
    assert_eq!(VariableByteIntegerEncoder::len(res), 3);
}

#[test]
fn test_encode_extra_small() {
    let encoded = VariableByteIntegerEncoder::encode(5);
    assert!(encoded.is_ok());
    let res = encoded.unwrap();
    assert_eq!(res, [0x05, 0x00, 0x00, 0x00]);
    assert_eq!(VariableByteIntegerEncoder::len(res), 1);
}

#[test]
fn test_encode_max() {
    let encoded = VariableByteIntegerEncoder::encode(288_435_455);
    assert!(encoded.is_err());
    assert_eq!(encoded.unwrap_err(), BufferError::EncodingError);
}
