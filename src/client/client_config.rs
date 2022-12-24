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
use rand_core::RngCore;

use crate::packet::v5::property::Property;
use crate::packet::v5::publish_packet::QualityOfService;
use crate::utils::types::{BinaryData, EncodedString};

#[derive(Clone, PartialEq)]
pub enum MqttVersion {
    MQTTv3,
    MQTTv5,
}
/// Client config is main configuration for the `MQTTClient` structure.
/// All of the properties are optional if they are not set they are not gonna
/// be used. Configuration contains also MQTTv5 properties. Generic constant
/// `MAX_PROPERTIES` sets the length for the properties Vec. User can insert
/// all the properties and client will automatically use variables that are
/// usable for the specific packet types. `mqtt_version` sets the version
/// of the MQTT protocol that is gonna be used. Config also expects the rng
/// implementation. This implementation is used for generating packet identifiers.
/// There is counting rng implementation in the `utils` module that can be used.
/// Examples of the configurations can be found in the integration tests.
#[derive(Clone)]
pub struct ClientConfig<'a, const MAX_PROPERTIES: usize, T: RngCore> {
    pub max_subscribe_qos: QualityOfService,
    pub keep_alive: u16,
    pub username_flag: bool,
    pub username: EncodedString<'a>,
    pub password_flag: bool,
    pub password: BinaryData<'a>,
    pub properties: Vec<Property<'a>, MAX_PROPERTIES>,
    pub max_packet_size: u32,
    pub mqtt_version: MqttVersion,
    pub rng: T,
    pub will_flag: bool,
    pub will_topic: EncodedString<'a>,
    pub will_payload: BinaryData<'a>,
    pub will_retain: bool,
    pub client_id: EncodedString<'a>,
}

impl<'a, const MAX_PROPERTIES: usize, T: RngCore> ClientConfig<'a, MAX_PROPERTIES, T> {
    pub fn new(version: MqttVersion, rng: T) -> Self {
        Self {
            max_subscribe_qos: QualityOfService::QoS0,
            keep_alive: 60,
            username_flag: false,
            username: EncodedString::new(),
            password_flag: false,
            password: BinaryData::new(),
            properties: Vec::<Property<'a>, MAX_PROPERTIES>::new(),
            max_packet_size: 265_000,
            mqtt_version: version,
            rng,
            will_flag: false,
            will_topic: EncodedString::new(),
            will_payload: BinaryData::new(),
            will_retain: false,
            client_id: EncodedString::new(),
        }
    }

    pub fn add_max_subscribe_qos(&mut self, qos: QualityOfService) {
        self.max_subscribe_qos = qos;
    }

    pub fn add_will(&mut self, topic: &'a str, payload: &'a [u8], retain: bool) {
        let mut topic_s = EncodedString::new();
        topic_s.string = topic;
        topic_s.len = topic.len() as u16;

        let mut payload_d = BinaryData::new();
        payload_d.bin = payload;
        payload_d.len = payload.len() as u16;

        self.will_flag = true;
        self.will_retain = retain;
        self.will_topic = topic_s;
        self.will_payload = payload_d;
    }

    /// Method adds the username array and also sets the username flag so client
    /// will use it for the authentication
    pub fn add_username(&mut self, username: &'a str) {
        let mut username_s: EncodedString = EncodedString::new();
        username_s.string = username;
        username_s.len = username.len() as u16;
        self.username_flag = true;
        self.username = username_s;
    }
    /// Method adds the password array and also sets the password flag so client
    /// will use it for the authentication
    pub fn add_password(&mut self, password: &'a str) {
        let mut password_s: BinaryData = BinaryData::new();
        password_s.bin = password.as_bytes();
        password_s.len = password_s.bin.len() as u16;
        self.password = password_s;
        self.password_flag = true;
    }

    /// Method adds the property to the properties Vec if there is still space. Otherwise do nothing.
    pub fn add_property(&mut self, prop: Property<'a>) {
        if self.properties.len() < MAX_PROPERTIES {
            self.properties.push(prop);
        }
    }

    /// Method encode the `max_packet_size` attribute as property to the properties Vec.
    pub fn add_max_packet_size_as_prop(&mut self) -> u32 {
        if self.properties.len() < MAX_PROPERTIES {
            let prop = Property::MaximumPacketSize(self.max_packet_size);
            self.properties.push(prop);
            return 5;
        }
        0
    }

    pub fn add_client_id(&mut self, client_id: &'a str) {
        let mut client_id_s = EncodedString::new();
        client_id_s.string = client_id;
        client_id_s.len = client_id.len() as u16;

        self.client_id = client_id_s
    }
}
