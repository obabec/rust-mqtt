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

use rand_core::RngCore;

use heapless::Vec;

use crate::{encoding::{BinaryData, EncodedString}, interface::Property};

#[derive(Clone, PartialEq)]
pub enum MqttVersion {
    MQTTv3,
    MQTTv5,
}

#[derive(Clone, Copy, Default, Debug)]
pub enum RetainHandling {
    #[default]
    AlwaysSend = 0,
    SendIfNotSubscribedBefore = 1,
    NeverSend = 2,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Default, Debug)]
pub enum QualityOfService {
    #[default]
    QoS0 = 0,
    QoS1 = 1,
    QoS2 = 2,
    INVALID = 3,
}

impl QualityOfService {
    pub fn into_publish_bits(&self) -> u8 {
        match self {
            Self::QoS0 => 0x00,
            Self::QoS1 => 0x02,
            Self::QoS2 => 0x04,
            Self::INVALID => 0x06,
        }
    }

    pub fn into_subscribe_bits(&self) -> u8 {
        match self {
            Self::QoS0 => 0x00,
            Self::QoS1 => 0x01,
            Self::QoS2 => 0x02,
            Self::INVALID => 0x03,
        }
    }

    pub fn from_publish_fixed_header(bits: u8) -> Self {
        let qos_bits = bits & 0x06;
        match qos_bits {
            0x00 => Self::QoS0,
            0x02 => Self::QoS1,
            0x04 => Self::QoS2,
            _ => Self::INVALID,
        }
    }

    pub fn from_subscribe_options(bits: u8) -> Self {
        let qos_bits = bits & 0x03;
        match qos_bits {
            0x00 => Self::QoS0,
            0x01 => Self::QoS1,
            0x02 => Self::QoS2,
            _ => Self::INVALID,
        }
    }

    pub fn from_suback_reason_code(bits: u8) -> Self {
        Self::from_subscribe_options(bits)
    }
}

/// Topic filter serves as public interface for topic selection and subscription options for `SUBSCRIBE` packet
#[derive(Clone, Copy, Debug)]
pub struct Topic<'a> {
    pub topic_name: &'a str,
    pub retain_handling: RetainHandling,
    pub retain_as_published: bool,
    pub no_local: bool,
    pub qos: QualityOfService,
}

impl<'a> Topic<'a> {
    /// Constructs a topic with the given name and defaults:
    /// - Retain Handling: always send
    /// - Retain As Published: false
    /// - No Local: false
    /// - Quality of Service: Quality of Service Level 0
    ///
    /// The defaults can be changed using builder methods
    pub fn new(topic_name: &'a str) -> Self {
        Self {
            topic_name,
            retain_handling: RetainHandling::default(),
            retain_as_published: bool::default(),
            no_local: bool::default(),
            qos: QualityOfService::default(),
        }
    }

    pub fn retain_handling(mut self, retain_handling: RetainHandling) -> Self {
        self.retain_handling = retain_handling;
        self
    }
    pub fn retain_as_published(mut self, retain_as_published: bool) -> Self {
        self.retain_as_published = retain_as_published;
        self
    }
    pub fn no_local(mut self, no_local: bool) -> Self {
        self.no_local = no_local;
        self
    }
    pub fn quality_of_service(mut self, quality_of_service: QualityOfService) -> Self {
        self.qos = quality_of_service;
        self
    }
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
            let _ = self.properties.push(prop);
        }
    }

    /// Method encode the `max_packet_size` attribute as property to the properties Vec.
    pub fn add_max_packet_size_as_prop(&mut self) -> u32 {
        if self.properties.len() < MAX_PROPERTIES {
            let prop = Property::MaximumPacketSize(self.max_packet_size);
            let _ = self.properties.push(prop);
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
