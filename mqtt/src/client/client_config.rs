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

use crate::packet::v5::property::Property;
use crate::packet::v5::publish_packet::QualityOfService;
use crate::utils::types::{BinaryData, EncodedString};

use heapless::Vec;
use rand_core::RngCore;

#[derive(Clone, PartialEq)]
pub enum MqttVersion {
    MQTTv3,
    MQTTv5,
}

#[derive(Clone)]
pub struct ClientConfig<'a, const MAX_PROPERTIES: usize, T: RngCore> {
    pub qos: QualityOfService,
    pub keep_alive: u16,
    pub username_flag: bool,
    pub username: EncodedString<'a>,
    pub password_flag: bool,
    pub password: BinaryData<'a>,
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
    pub max_packet_size: u32,
    pub mqtt_version: MqttVersion,
    pub rng: T,
}

impl<'a, const MAX_PROPERTIES: usize, T: RngCore> ClientConfig<'a, MAX_PROPERTIES, T> {
    pub fn new(version: MqttVersion, rng: T) -> Self {
        Self {
            qos: QualityOfService::QoS0,
            keep_alive: 60,
            username_flag: false,
            username: EncodedString::new(),
            password_flag: false,
            password: BinaryData::new(),
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            max_packet_size: 265_000,
            mqtt_version: version,
            rng,
        }
    }

    pub fn add_qos(&mut self, qos: QualityOfService) {
        self.qos = qos;
    }

    pub fn add_username(&mut self, username: &'a str) {
        let mut username_s: EncodedString = EncodedString::new();
        username_s.string = username;
        username_s.len = username.len() as u16;
        self.username_flag = true;
        self.username = username_s;
    }

    pub fn add_password(&mut self, password: &'a str) {
        let mut password_s: BinaryData = BinaryData::new();
        password_s.bin = password.as_bytes();
        password_s.len = password_s.bin.len() as u16;
        self.password = password_s;
        self.password_flag = true;
    }

    pub fn add_property(&mut self, prop: Property<'a>) {
        if self.properties.len() < MAX_PROPERTIES {
            self.properties.push(prop);
        }
    }

    pub fn add_max_packet_size_as_prop(&mut self) -> u32 {
        if self.properties.len() < MAX_PROPERTIES {
            let prop = Property::MaximumPacketSize(self.max_packet_size);
            self.properties.push(prop);
            return 5;
        }
        return 0;
    }
}
