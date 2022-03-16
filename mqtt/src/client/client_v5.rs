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

use crate::client::client_config::ClientConfig;
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
use crate::packet::v5::suback_packet::SubackPacket;
use crate::packet::v5::subscription_packet::SubscriptionPacket;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::rng_generator::CountingRng;
use crate::utils::types::BufferError;

use heapless::Vec;
use rand_core::RngCore;
use crate::network::NetworkError::Connection;

pub struct MqttClientV5<'a, T, const MAX_PROPERTIES: usize> {
    connection: Option<T>,
    buffer: &'a mut [u8],
    buffer_len: usize,
    recv_buffer: &'a mut [u8],
    recv_buffer_len: usize,
    rng: CountingRng,
    config: ClientConfig<'a, MAX_PROPERTIES>,
}

impl<'a, T, const MAX_PROPERTIES: usize> MqttClientV5<'a, T, MAX_PROPERTIES>
where
    T: NetworkConnection,
{
    pub fn new(
        network_driver: T,
        buffer: &'a mut [u8],
        buffer_len: usize,
        recv_buffer: &'a mut [u8],
        recv_buffer_len: usize,
        config: ClientConfig<'a, MAX_PROPERTIES>,
    ) -> Self {
        Self {
            connection: Some(network_driver),
            buffer,
            buffer_len,
            recv_buffer,
            recv_buffer_len,
            rng: CountingRng(50),
            config,
        }
    }

    pub async fn connect_to_broker<'b>(&'b mut self) -> Result<(), ReasonCode> {
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
            connect.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        let mut conn = self.connection.as_mut().unwrap();
        trace!("Sending connect");
        conn.send(&self.buffer[0..len.unwrap()]).await?;

        //connack
        let reason: Result<u8, BufferError> = {
            trace!("Waiting for connack");
            conn.receive(self.recv_buffer).await?;
            let mut packet = ConnackPacket::<'b, MAX_PROPERTIES>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len)) {
                if err == BufferError::PacketTypeMismatch {
                    let mut disc = DisconnectPacket::<'b, MAX_PROPERTIES>::new();
                    if disc
                        .decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len))
                        .is_ok()
                    {
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

    pub async fn disconnect<'b>(&'b mut self) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        trace!("Creating disconnect packet!");
        let mut disconnect = DisconnectPacket::<'b, MAX_PROPERTIES>::new();
        let len = disconnect.encode(self.buffer, self.buffer_len);
        if let Err(err) = len {
            warn!("[DECODE ERR]: {}", err);
            self.connection.take().unwrap().close().await?;
            return Err(ReasonCode::BuffError);
        }

        if let Err(e) = conn.send(&self.buffer[0..len.unwrap()]).await {
            warn!("Could not send DISCONNECT packet");
        }

        if let Err(e) = self.connection.take().unwrap().close().await {
            warn!("Could not close the TCP handle");
            return Err(e);
        } else {
            trace!("Closed TCP handle");
        }
        Ok(())
    }

    pub async fn send_message<'b>(
        &'b mut self,
        topic_name: &'b str,
        message: &'b str,
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let mut conn = self.connection.as_mut().unwrap();
        let identifier: u16 = self.rng.next_u32() as u16;
        let len = {
            let mut packet = PublishPacket::<'b, MAX_PROPERTIES>::new();
            packet.add_topic_name(topic_name);
            packet.add_qos(self.config.qos);
            packet.add_identifier(identifier);
            packet.add_message(message.as_bytes());
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
                conn.receive(self.recv_buffer).await?;
                let mut packet = PubackPacket::<'b, MAX_PROPERTIES>::new();
                if let Err(err) = packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len))
                {
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

    pub async fn subscribe_to_topics<'b, const TOPICS: usize>(
        &'b mut self,
        topic_names: &'b Vec<&'b str, TOPICS>,
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let mut conn = self.connection.as_mut().unwrap();
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
            conn.receive(self.recv_buffer).await?;

            let mut packet = SubackPacket::<'b, TOPICS, MAX_PROPERTIES>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len)) {
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
            if *reasons.get(i).unwrap() != (<QualityOfService as Into<u8>>::into(self.config.qos) >> 1) {
                return Err(ReasonCode::from(*reasons.get(i).unwrap()));
            }
            i = i + 1;
        }
        Ok(())
    }

    pub async fn subscribe_to_topic<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let mut conn = self.connection.as_mut().unwrap();
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
            conn.receive(self.recv_buffer).await?;

            let mut packet = SubackPacket::<'b, 5, MAX_PROPERTIES>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len)) {
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

    pub async fn receive_message<'b>(&'b mut self) -> Result<&'b [u8], ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let mut conn = self.connection.as_mut().unwrap();
        conn.receive(self.recv_buffer).await?;
        let mut packet = PublishPacket::<'b, 5>::new();
        if let Err(err) =
            packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len))
        {
            if err == BufferError::PacketTypeMismatch {
                let mut disc = DisconnectPacket::<'b, 5>::new();
                if disc
                    .decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len))
                    .is_ok()
                {
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
                let len = puback.encode(self.buffer, self.buffer_len);
                if let Err(err) = len {
                    error!("[DECODE ERR]: {}", err);
                    return Err(ReasonCode::BuffError);
                }
                conn.send(&self.buffer[0..len.unwrap()]).await?;
            }
        }

        return Ok(packet.message.unwrap());
    }

    pub async fn send_ping<'b>(&'b mut self) -> Result<(), ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let mut conn = self.connection.as_mut().unwrap();
        let len = {
            let mut packet = PingreqPacket::new();
            packet.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        conn.send(&self.buffer[0..len.unwrap()]).await?;

        conn.receive(self.recv_buffer).await?;
        let mut packet = PingrespPacket::new();
        if let Err(err) = packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len)) {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        } else {
            Ok(())
        }
    }
}
