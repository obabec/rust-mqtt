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

pub mod client;
pub mod client_trait;
pub mod raw;

use embedded_io::ReadReady;
use embedded_io_async::{Read, Write};
use heapless::Vec;
use rand_core::RngCore;

use crate::{
    client::raw::{Event, RawMqttClient},
    interface::{ClientConfig, QualityOfService, ReasonCode, Topic},
};

pub struct MqttClient<'a, T, const MAX_PROPERTIES: usize, R: RngCore>
where
    T: Read + Write,
{
    raw: RawMqttClient<'a, T, MAX_PROPERTIES, R>,
}

impl<'a, T, const MAX_PROPERTIES: usize, R> MqttClient<'a, T, MAX_PROPERTIES, R>
where
    T: Read + Write,
    R: RngCore,
{
    pub fn new(
        network_driver: T,
        buffer: &'a mut [u8],
        recv_buffer: &'a mut [u8],
        config: ClientConfig<'a, MAX_PROPERTIES, R>,
    ) -> Self {
        Self {
            raw: RawMqttClient::new(network_driver, buffer, recv_buffer, config),
        }
    }

    /// Method allows client connect to server. Client is connecting to the specified broker
    /// in the `ClientConfig`. Method selects proper implementation of the MQTT version based on the config.
    /// If the connection to the broker fails, method returns Err variable that contains
    /// Reason codes returned from the broker.
    pub async fn connect_to_broker<'b>(&'b mut self) -> Result<(), ReasonCode> {
        self.raw.connect_to_broker().await?;

        match self.raw.poll::<0>().await? {
            Event::Connack => Ok(()),
            Event::Disconnect(reason) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }

    /// Method allows client disconnect from the server. Client disconnects from the specified broker
    /// in the `ClientConfig`. Method selects proper implementation of the MQTT version based on the config.
    /// If the disconnect from the broker fails, method returns Err variable that contains
    /// Reason codes returned from the broker.
    pub async fn disconnect<'b>(&'b mut self) -> Result<(), ReasonCode> {
        self.raw.disconnect().await?;
        Ok(())
    }

    /// Method allows sending message to broker specified from the ClientConfig. Client sends the
    /// message from the parameter `message` to the topic `topic_name` on the broker
    /// specified in the ClientConfig. If the send fails method returns Err with reason code
    /// received by broker.
    pub async fn send_message<'b>(
        &'b mut self,
        topic_name: &'b str,
        message: &'b [u8],
        qos: QualityOfService,
        retain: bool,
    ) -> Result<(), ReasonCode> {
        let identifier = self
            .raw
            .send_message(topic_name, message, qos, retain)
            .await?;

        // QoS1
        if qos == QualityOfService::QoS1 {
            match self.raw.poll::<0>().await? {
                Event::Puback(ack_identifier, matching_subscriber) => {
                    if !matching_subscriber {
                        Err(ReasonCode::NoMatchingSubscribers)
                    } else if identifier == ack_identifier {
                        Ok(())
                    } else {
                        Err(ReasonCode::PacketIdentifierNotFound)
                    }
                }
                Event::Disconnect(reason) => Err(reason),
                // If an application message comes at this moment, it is lost.
                _ => Err(ReasonCode::ImplementationSpecificError),
            }
        } else {
            Ok(())
        }
    }

    /// Method allows client subscribe to multiple topics specified in the parameter
    /// `topic_names` on the broker specified in the `ClientConfig`. Generics `TOPICS`
    /// sets the value of the `topics_names` vector. MQTT protocol implementation
    /// is selected automatically.
    pub async fn subscribe_to_topics<'b, const TOPICS: usize>(
        &'b mut self,
        topics: &'b Vec<Topic<'b>, TOPICS>,
    ) -> Result<(), ReasonCode> {
        let identifier = self.raw.subscribe_to_topics(topics).await?;

        match self.raw.poll::<TOPICS>().await? {
            Event::Suback(ack_identifier) => {
                if identifier == ack_identifier {
                    Ok(())
                } else {
                    Err(ReasonCode::PacketIdentifierNotFound)
                }
            }
            Event::Disconnect(reason) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }

    /// Method allows client unsubscribe from the topic specified in the parameter
    /// `topic_name` on the broker from the `ClientConfig`. MQTT protocol implementation
    /// is selected automatically.
    pub async fn unsubscribe_from_topic<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<(), ReasonCode> {
        let identifier = self.raw.unsubscribe_from_topic(topic_name).await?;

        match self.raw.poll::<0>().await? {
            Event::Unsuback(ack_identifier) => {
                if identifier == ack_identifier {
                    Ok(())
                } else {
                    Err(ReasonCode::PacketIdentifierNotFound)
                }
            }
            Event::Disconnect(reason) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }

    /// Method allows client subscribe to multiple topics specified in the parameter
    /// `topic_name` on the broker specified in the `ClientConfig`. MQTT protocol implementation
    /// is selected automatically.
    pub async fn subscribe_to_topic<'b>(&'b mut self, topic: Topic<'b>) -> Result<(), ReasonCode> {
        let mut topics = Vec::<Topic<'b>, 1>::new();
        topics.push(topic).unwrap();

        let identifier = self.raw.subscribe_to_topics(&topics).await?;

        match self.raw.poll::<1>().await? {
            Event::Suback(ack_identifier) => {
                if identifier == ack_identifier {
                    Ok(())
                } else {
                    Err(ReasonCode::PacketIdentifierNotFound)
                }
            }
            Event::Disconnect(reason) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }

    /// Method allows client receive a message. The work of this method strictly depends on the
    /// network implementation passed in the `ClientConfig`. It expects the PUBLISH packet
    /// from the broker.
    pub async fn receive_message<'b>(&'b mut self) -> Result<(&'b str, &'b [u8]), ReasonCode> {
        match self.raw.poll::<0>().await? {
            Event::Message(topic, payload) => Ok((topic, payload)),
            Event::Disconnect(reason) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }

    /// Method allows client send PING message to the broker specified in the `ClientConfig`.
    /// If there is expectation for long running connection. Method should be executed
    /// regularly by the timer that counts down the session expiry interval.
    pub async fn send_ping<'b>(&'b mut self) -> Result<(), ReasonCode> {
        self.raw.send_ping().await?;

        match self.raw.poll::<0>().await? {
            Event::Pingresp => Ok(()),
            Event::Disconnect(reason) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }
}

impl<'a, T, const MAX_PROPERTIES: usize, R> MqttClient<'a, T, MAX_PROPERTIES, R>
where
    T: Read + Write + ReadReady,
    R: RngCore,
{
    /// Receive a message if one is ready. The work of this method strictly depends on the
    /// network implementation passed in the `ClientConfig`. It expects the PUBLISH packet
    /// from the broker.
    pub async fn receive_message_if_ready<'b>(
        &'b mut self,
    ) -> Result<Option<(&'b str, &'b [u8])>, ReasonCode> {
        match self.raw.poll_if_ready::<0>().await? {
            None => Ok(None),
            Some(Event::Message(topic, payload)) => Ok(Some((topic, payload))),
            Some(Event::Disconnect(reason)) => Err(reason),
            // If an application message comes at this moment, it is lost.
            _ => Err(ReasonCode::ImplementationSpecificError),
        }
    }
}
