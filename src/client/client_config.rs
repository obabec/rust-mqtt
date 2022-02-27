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


use crate::packet::publish_packet::QualityOfService;
use crate::utils::buffer_reader::{BinaryData, EncodedString};

pub struct ClientConfig<'a> {
    pub qos: QualityOfService,
    pub username_flag: bool,
    pub username: EncodedString<'a>,
    pub password_flag: bool,
    pub password: BinaryData<'a>
}

impl ClientConfig<'a> {
    pub fn new() -> Self {
        Self {
            qos: QualityOfService::QoS0,
            username_flag: false,
            username: EncodedString::new(),
            password_flag: false,
            password: BinaryData::new(),
        }
    }

    pub fn add_qos(& mut self, qos: QualityOfService) {
        self.qos = qos;
    }

    pub fn add_username(& mut self, username: &'a str) {
        let mut username_s: EncodedString = EncodedString::new();
        username_s.string = username;
        username_s.len = username.len() as u16;
        self.username_flag = true;
        self.username = username_s;
    }

    pub fn add_password(& mut self, password: &'a str) {
        let mut password_s: BinaryData = BinaryData::new();
        password_s.bin = password.as_bytes();
        password_s.len = password_s.bin.len() as u16;
        self.password = password_s;
        self.password_flag = true;
    }
}