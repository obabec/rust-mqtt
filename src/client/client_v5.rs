use crate::client::client_config::ClientConfig;
use crate::network::network_trait::Network;
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

pub struct MqttClientV5<'a, T, const MAX_PROPERTIES: usize> {
    network_driver: &'a mut T,
    buffer: &'a mut [u8],
    buffer_len: usize,
    recv_buffer: &'a mut [u8],
    recv_buffer_len: usize,
    rng: CountingRng,
    config: ClientConfig<'a>,
}

impl<'a, T, const MAX_PROPERTIES: usize> MqttClientV5<'a, T, MAX_PROPERTIES>
where
    T: Network,
{
    pub fn new(
        network_driver: &'a mut T,
        buffer: &'a mut [u8],
        buffer_len: usize,
        recv_buffer: &'a mut [u8],
        recv_buffer_len: usize,
        config: ClientConfig<'a>,
    ) -> Self {
        Self {
            network_driver,
            buffer,
            buffer_len,
            recv_buffer,
            recv_buffer_len,
            rng: CountingRng(50),
            config,
        }
    }

    pub async fn connect_to_broker<'b>(&'b mut self) -> Result<(), ReasonCode> {
        let len = {
            let mut connect = ConnectPacket::<'b, 3, 0>::clean();
            if self.config.username_flag {
                connect.add_username(&self.config.username);
            }
            if self.config.password_flag {
                connect.add_password(&self.config.password)
            }
            connect.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        self.network_driver.send(self.buffer, len.unwrap()).await?;

        //connack
        let reason: Result<u8, BufferError> = {
            self.network_driver.receive(self.buffer).await?;
            let mut packet = ConnackPacket::<'b, 5>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, self.buffer_len)) {
                Err(err)
            } else {
                Ok(packet.connect_reason_code)
            }
        };

        if let Err(err) = reason {
            log::error!("[DECODE ERR]: {}", err);
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
        let mut disconnect = DisconnectPacket::<'b, 5>::new();
        let len = disconnect.encode(self.buffer, self.buffer_len);
        if let Err(err) = len {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        self.network_driver.send(self.buffer, len.unwrap()).await?;
        Ok(())
    }

    pub async fn send_message<'b>(
        &'b mut self,
        topic_name: &'b str,
        message: &'b str,
    ) -> Result<(), ReasonCode> {
        let identifier: u16 = self.rng.next_u32() as u16;
        let len = {
            let mut packet = PublishPacket::<'b, 5>::new();
            packet.add_topic_name(topic_name);
            packet.add_qos(self.config.qos);
            packet.add_identifier(identifier);
            packet.add_message(message.as_bytes());
            packet.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        self.network_driver.send(self.buffer, len.unwrap()).await?;

        //QoS1
        if <QualityOfService as Into<u8>>::into(self.config.qos)
            == <QualityOfService as Into<u8>>::into(QoS1)
        {
            let reason: Result<[u16; 2], BufferError> = {
                self.network_driver.receive(self.buffer).await?;
                let mut packet = PubackPacket::<'b, 5>::new();
                if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, self.buffer_len))
                {
                    Err(err)
                } else {
                    Ok([packet.packet_identifier, packet.reason_code as u16])
                }
            };

            if let Err(err) = reason {
                log::error!("[DECODE ERR]: {}", err);
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
        let len = {
            let mut subs = SubscriptionPacket::<'b, TOPICS, 1>::new();
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
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        self.network_driver.send(self.buffer, len.unwrap()).await?;

        let reason: Result<Vec<u8, TOPICS>, BufferError> = {
            self.network_driver.receive(self.buffer).await?;

            let mut packet = SubackPacket::<'b, TOPICS, 5>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, self.buffer_len)) {
                Err(err)
            } else {
                Ok(packet.reason_codes)
            }
        };

        if let Err(err) = reason {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        let reasons = reason.unwrap();
        let mut i = 0;
        loop {
            if i == TOPICS {
                break;
            }
            if *reasons.get(i).unwrap() != self.config.qos.into() {
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
        let len = {
            let mut subs = SubscriptionPacket::<'b, 1, 1>::new();
            subs.add_new_filter(topic_name, self.config.qos);
            subs.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        self.network_driver.send(self.buffer, len.unwrap()).await?;

        let reason: Result<u8, BufferError> = {
            self.network_driver.receive(self.buffer).await?;

            let mut packet = SubackPacket::<'b, 5, 5>::new();
            if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, self.buffer_len)) {
                Err(err)
            } else {
                Ok(*packet.reason_codes.get(0).unwrap())
            }
        };

        if let Err(err) = reason {
            log::error!("[DECODE ERR]: {}", err);
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
        self.network_driver.receive(self.recv_buffer).await?;
        let mut packet = PublishPacket::<'b, 5>::new();
        if let Err(err) =
            packet.decode(&mut BuffReader::new(self.recv_buffer, self.recv_buffer_len))
        {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        if (packet.fixed_header & 0x06)
            == <QualityOfService as Into<u8>>::into(QualityOfService::QoS1)
        {
            let mut puback = PubackPacket::<'b, 5>::new();
            puback.packet_identifier = packet.packet_identifier;
            puback.reason_code = 0x00;
            {
                let len = puback.encode(self.buffer, self.buffer_len);
                if let Err(err) = len {
                    log::error!("[DECODE ERR]: {}", err);
                    return Err(ReasonCode::BuffError);
                }
                self.network_driver.send(self.buffer, len.unwrap()).await?;
            }
        }

        return Ok(packet.message.unwrap());
    }

    pub async fn send_ping<'b>(&'b mut self) -> Result<(), ReasonCode> {
        let len = {
            let mut packet = PingreqPacket::new();
            packet.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        self.network_driver.send(self.buffer, len.unwrap()).await?;

        self.network_driver.receive(self.buffer).await?;
        let mut packet = PingrespPacket::new();
        if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, self.buffer_len)) {
            log::error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        } else {
            Ok(())
        }
    }
}
