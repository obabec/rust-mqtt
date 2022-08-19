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

use embedded_io::asynch::{Read, Write};
use heapless::Vec;
use rand_core::RngCore;

use crate::client::client_config::{ClientConfig, MqttVersion};
use crate::encoding::variable_byte_integer::{VariableByteInteger, VariableByteIntegerDecoder};
use crate::network::NetworkConnection;
use crate::packet::v5::connack_packet::ConnackPacket;
use crate::packet::v5::connect_packet::ConnectPacket;
use crate::packet::v5::disconnect_packet::DisconnectPacket;
use crate::packet::v5::mqtt_packet::Packet;
use crate::packet::v5::pingreq_packet::PingreqPacket;
use crate::packet::v5::pingresp_packet::PingrespPacket;
use crate::packet::v5::puback_packet::PubackPacket;
use crate::packet::v5::publish_packet::QualityOfService::QoS1;
use crate::packet::v5::publish_packet::{PublishPacket, QualityOfService};
use crate::packet::v5::reason_codes::ReasonCode;
use crate::packet::v5::reason_codes::ReasonCode::{BuffError, NetworkError};
use crate::packet::v5::suback_packet::SubackPacket;
use crate::packet::v5::subscription_packet::SubscriptionPacket;
use crate::packet::v5::unsuback_packet::UnsubackPacket;
use crate::packet::v5::unsubscription_packet::UnsubscriptionPacket;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;
use crate::utils::types::BufferError;

pub struct MqttClient<'a, T, const MAX_PROPERTIES: usize, R: RngCore>
where
    T: Read + Write,
{
    connection: Option<NetworkConnection<T>>,
    buffer: &'a mut [u8],
    buffer_len: usize,
    recv_buffer: &'a mut [u8],
    recv_buffer_len: usize,
    config: ClientConfig<'a, MAX_PROPERTIES, R>,
}

impl<'a, T, const MAX_PROPERTIES: usize, R> MqttClient<'a, T, MAX_PROPERTIES, R>
where
    T: Read + Write,
    R: RngCore,
{
    pub fn new(
        network_driver: T,
        buffer: &'a mut [u8],
        buffer_len: usize,
        recv_buffer: &'a mut [u8],
        recv_buffer_len: usize,
        config: ClientConfig<'a, MAX_PROPERTIES, R>,
    ) -> Self {
        Self {
            connection: Some(NetworkConnection::new(network_driver)),
            buffer,
            buffer_len,
            recv_buffer,
            recv_buffer_len,
            config,
        }
    }

    async fn connect_to_broker_v5<'b>(&'b mut self) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let len = {
            let mut connect = ConnectPacket::<'b, MAX_PROPERTIES, 0>::new();
            connect.keep_alive = self.config.keep_alive;
            self.config.add_max_packet_size_as_prop();
            connect.property_len = connect.add_properties(&self.config.properties);
            if self.config.username_flag {
                connect.add_username(&self.config.username);
            }
            if self.config.password_flag {
                connect.add_password(&self.config.password)
            }
            if self.config.will_flag {
                connect.add_will(
                    &self.config.will_topic,
                    &self.config.will_payload,
                    self.config.will_retain,
                )
            }
            connect.add_client_id(&self.config.client_id);
            connect.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        let conn = self.connection.as_mut().unwrap();
        trace!("Sending connect");
        conn.send(&self.buffer[0..len.unwrap()]).await?;

        //connack
        let reason: Result<u8, BufferError> = {
            trace!("Waiting for connack");

            let read =
                { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };

            let mut packet = ConnackPacket::<'b, MAX_PROPERTIES>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                if err == BufferError::PacketTypeMismatch {
                    let mut disc = DisconnectPacket::<'b, MAX_PROPERTIES>::new();
                    if disc.decode(&mut BuffReader::new(self.buffer, read)).is_ok() {
                        error!("Client was disconnected with reason: ");
                        return Err(ReasonCode::from(disc.disconnect_reason));
                    }
                }
                Err(err)
            } else {
                Ok(packet.connect_reason_code)
            }
        };

        if let Err(err) = reason {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        let res = reason.unwrap();
        if res != 0x00 {
            return Err(ReasonCode::from(res));
        } else {
            Ok(())
        }
    }

    /// Method allows client connect to server. Client is connecting to the specified broker
    /// in the `ClientConfig`. Method selects proper implementation of the MQTT version based on the config.
    /// If the connection to the broker fails, method returns Err variable that contains
    /// Reason codes returned from the broker.
    pub async fn connect_to_broker<'b>(&'b mut self) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.connect_to_broker_v5().await,
        }
    }

    async fn disconnect_v5<'b>(&'b mut self) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        trace!("Creating disconnect packet!");
        let mut disconnect = DisconnectPacket::<'b, MAX_PROPERTIES>::new();
        let len = disconnect.encode(self.buffer, self.buffer_len);
        if let Err(err) = len {
            warn!("[DECODE ERR]: {}", err);
            let _ = self.connection.take();
            return Err(ReasonCode::BuffError);
        }

        if let Err(_e) = conn.send(&self.buffer[0..len.unwrap()]).await {
            warn!("Could not send DISCONNECT packet");
        }

        // Drop connection
        let _ = self.connection.take();
        Ok(())
    }

    /// Method allows client disconnect from the server. Client disconnects from the specified broker
    /// in the `ClientConfig`. Method selects proper implementation of the MQTT version based on the config.
    /// If the disconnect from the broker fails, method returns Err variable that contains
    /// Reason codes returned from the broker.
    pub async fn disconnect<'b>(&'b mut self) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.disconnect_v5().await,
        }
    }

    async fn send_message_v5<'b>(
        &'b mut self,
        topic_name: &'b str,
        message: &'b [u8],
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let identifier: u16 = self.config.rng.next_u32() as u16;
        //self.rng.next_u32() as u16;
        let len = {
            let mut packet = PublishPacket::<'b, MAX_PROPERTIES>::new();
            packet.add_topic_name(topic_name);
            packet.add_qos(self.config.qos);
            packet.add_identifier(identifier);
            packet.add_message(message);
            packet.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        trace!("Sending message");
        conn.send(&self.buffer[0..len.unwrap()]).await?;

        // QoS1
        if <QualityOfService as Into<u8>>::into(self.config.qos)
            == <QualityOfService as Into<u8>>::into(QoS1)
        {
            let reason: Result<[u16; 2], BufferError> = {
                trace!("Waiting for ack");
                let read =
                    receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await?;
                trace!("[PUBACK] Received packet with len");
                let mut packet = PubackPacket::<'b, MAX_PROPERTIES>::new();
                if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                    Err(err)
                } else {
                    Ok([packet.packet_identifier, packet.reason_code as u16])
                }
            };

            if let Err(err) = reason {
                error!("[DECODE ERR]: {}", err);
                return Err(ReasonCode::BuffError);
            }

            let res = reason.unwrap();
            if identifier != res[0] {
                return Err(ReasonCode::PacketIdentifierNotFound);
            }

            if res[1] != 0 {
                return Err(ReasonCode::from(res[1] as u8));
            }
        }
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
    ) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.send_message_v5(topic_name, message).await,
        }
    }

    async fn subscribe_to_topics_v5<'b, const TOPICS: usize>(
        &'b mut self,
        topic_names: &'b Vec<&'b str, TOPICS>,
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let len = {
            let mut subs = SubscriptionPacket::<'b, TOPICS, MAX_PROPERTIES>::new();
            let mut i = 0;
            loop {
                if i == TOPICS {
                    break;
                }
                subs.add_new_filter(topic_names.get(i).unwrap(), self.config.qos);
                i = i + 1;
            }
            subs.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        conn.send(&self.buffer[0..len.unwrap()]).await?;

        let reason: Result<Vec<u8, TOPICS>, BufferError> = {
            let read =
                { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };

            let mut packet = SubackPacket::<'b, TOPICS, MAX_PROPERTIES>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                Err(err)
            } else {
                Ok(packet.reason_codes)
            }
        };

        if let Err(err) = reason {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        let reasons = reason.unwrap();
        let mut i = 0;
        loop {
            if i == TOPICS {
                break;
            }
            if *reasons.get(i).unwrap()
                != (<QualityOfService as Into<u8>>::into(self.config.qos) >> 1)
            {
                return Err(ReasonCode::from(*reasons.get(i).unwrap()));
            }
            i = i + 1;
        }
        Ok(())
    }

    /// Method allows client subscribe to multiple topics specified in the parameter
    /// `topic_names` on the broker specified in the `ClientConfig`. Generics `TOPICS`
    /// sets the value of the `topics_names` vector. MQTT protocol implementation
    /// is selected automatically.
    pub async fn subscribe_to_topics<'b, const TOPICS: usize>(
        &'b mut self,
        topic_names: &'b Vec<&'b str, TOPICS>,
    ) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.subscribe_to_topics_v5(topic_names).await,
        }
    }

    /// Method allows client unsubscribe from the topic specified in the parameter
    /// `topic_name` on the broker from the `ClientConfig`. MQTT protocol implementation
    /// is selected automatically.
    pub async fn unsubscribe_from_topic<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.unsubscribe_from_topic_v5(topic_name).await,
        }
    }

    async fn unsubscribe_from_topic_v5<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();

        let len = {
            let mut unsub = UnsubscriptionPacket::<'b, 1, MAX_PROPERTIES>::new();
            unsub.packet_identifier = self.config.rng.next_u32() as u16;
            unsub.add_new_filter(topic_name);
            unsub.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        conn.send(&self.buffer[0..len.unwrap()]).await?;

        let reason: Result<u8, BufferError> = {
            let read =
                { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };
            let mut packet = UnsubackPacket::<'b, 1, MAX_PROPERTIES>::new();

            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                Err(err)
            } else {
                Ok(*packet.reason_codes.get(0).unwrap())
            }
        };

        if let Err(err) = reason {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        Ok(())
    }

    async fn subscribe_to_topic_v5<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let len = {
            let mut subs = SubscriptionPacket::<'b, 1, MAX_PROPERTIES>::new();
            subs.add_new_filter(topic_name, self.config.qos);
            subs.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        conn.send(&self.buffer[0..len.unwrap()]).await?;

        let reason: Result<u8, BufferError> = {
            let read =
                { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };

            let mut packet = SubackPacket::<'b, 1, MAX_PROPERTIES>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                Err(err)
            } else {
                Ok(*packet.reason_codes.get(0).unwrap())
            }
        };

        if let Err(err) = reason {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        let res = reason.unwrap();
        if res != (<QualityOfService as Into<u8>>::into(self.config.qos) >> 1) {
            Err(ReasonCode::from(res))
        } else {
            Ok(())
        }
    }

    /// Method allows client subscribe to multiple topics specified in the parameter
    /// `topic_name` on the broker specified in the `ClientConfig`. MQTT protocol implementation
    /// is selected automatically.
    pub async fn subscribe_to_topic<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.subscribe_to_topic_v5(topic_name).await,
        }
    }

    async fn receive_message_v5<'b>(&'b mut self) -> Result<(&'b str, &'b [u8]), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let read = { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };

        let mut packet = PublishPacket::<'b, 5>::new();
        if let Err(err) = { packet.decode(&mut BuffReader::new(self.buffer, read)) } {
            if err == BufferError::PacketTypeMismatch {
                let mut disc = DisconnectPacket::<'b, 5>::new();
                if disc.decode(&mut BuffReader::new(self.buffer, read)).is_ok() {
                    error!("Client was disconnected with reason: ");
                    return Err(ReasonCode::from(disc.disconnect_reason));
                }
            }
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        if (packet.fixed_header & 0x06)
            == <QualityOfService as Into<u8>>::into(QualityOfService::QoS1)
        {
            let mut puback = PubackPacket::<'b, MAX_PROPERTIES>::new();
            puback.packet_identifier = packet.packet_identifier;
            puback.reason_code = 0x00;
            {
                let len = { puback.encode(self.recv_buffer, self.recv_buffer_len) };
                if let Err(err) = len {
                    error!("[DECODE ERR]: {}", err);
                    return Err(ReasonCode::BuffError);
                }
                conn.send(&self.recv_buffer[0..len.unwrap()]).await?;
            }
        }

        return Ok((packet.topic_name.string, packet.message.unwrap()));
    }

    /// Method allows client receive a message. The work of this method strictly depends on the
    /// network implementation passed in the `ClientConfig`. It expects the PUBLISH packet
    /// from the broker.
    pub async fn receive_message<'b>(&'b mut self) -> Result<(&'b str, &'b [u8]), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.receive_message_v5().await,
        }
    }

    async fn send_ping_v5<'b>(&'b mut self) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let len = {
            let mut packet = PingreqPacket::new();
            packet.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        conn.send(&self.buffer[0..len.unwrap()]).await?;

        let read = { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };
        let mut packet = PingrespPacket::new();
        if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        } else {
            Ok(())
        }
    }

    /// Method allows client send PING message to the broker specified in the `ClientConfig`.
    /// If there is expectation for long running connection. Method should be executed
    /// regularly by the timer that counts down the session expiry interval.
    pub async fn send_ping<'b>(&'b mut self) -> Result<(), ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.send_ping_v5().await,
        }
    }
}

#[cfg(not(feature = "tls"))]
async fn receive_packet<'c, T: Read + Write>(
    buffer: &mut [u8],
    buffer_len: usize,
    recv_buffer: &mut [u8],
    conn: &'c mut NetworkConnection<T>,
) -> Result<usize, ReasonCode> {
    let target_len: usize;
    let mut rem_len: Result<VariableByteInteger, ()>;
    let mut writer = BuffWriter::new(buffer, buffer_len);
    let mut i = 0;

    // Get len of packet
    trace!("Reading lenght of packet");
    loop {
        trace!("    Reading in loop!");
        let len: usize = conn
            .receive(&mut recv_buffer[writer.position..(writer.position + 1)])
            .await?;
        trace!("    Received data!");
        if len == 0 {
            trace!("Zero byte len packet received, dropping connection.");
            return Err(NetworkError);
        }
        i = i + len;
        if let Err(_e) = writer.insert_ref(len, &recv_buffer[writer.position..i]) {
            error!("Error occurred during write to buffer!");
            return Err(ReasonCode::BuffError);
        }
        if i > 1 {
            rem_len = writer.get_rem_len();
            if rem_len.is_ok() {
                break;
            }
            if i >= 5 {
                error!("Could not read len of packet!");
                return Err(NetworkError);
            }
        }
    }
    trace!("Lenght done!");
    let rem_len_len = i;
    i = 0;
    if let Ok(l) = VariableByteIntegerDecoder::decode(rem_len.unwrap()) {
        trace!("Reading packet with target len {}", l);
        target_len = l as usize;
    } else {
        error!("Could not decode len of packet!");
        return Err(BuffError);
    }

    loop {
        if writer.position == target_len + rem_len_len {
            trace!("Received packet with len: {}", (target_len + rem_len_len));
            return Ok(target_len + rem_len_len);
        }
        let len: usize = conn
            .receive(&mut recv_buffer[writer.position..writer.position + (target_len - i)])
            .await?;
        i = i + len;
        if let Err(_e) =
            writer.insert_ref(len, &recv_buffer[writer.position..(writer.position + i)])
        {
            error!("Error occurred during write to buffer!");
            return Err(BuffError);
        }
    }
}

#[cfg(feature = "tls")]
async fn receive_packet<'c, T: Read + Write>(
    buffer: &mut [u8],
    buffer_len: usize,
    recv_buffer: &mut [u8],
    conn: &'c mut NetworkConnection<T>,
) -> Result<usize, ReasonCode> {
    trace!("Reading packet");
    let mut writer = BuffWriter::new(buffer, buffer_len);
    let len = conn.receive(recv_buffer).await?;
    if let Err(_e) = writer.insert_ref(len, &recv_buffer[writer.position..(writer.position + len)])
    {
        error!("Error occurred during write to buffer!");
        return Err(BuffError);
    }
    Ok(len)
}
