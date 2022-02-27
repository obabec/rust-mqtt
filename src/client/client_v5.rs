use core::future::Future;
use embassy::traits::rng;
use rand_core::RngCore;
use crate::client::client_config::ClientConfig;

use crate::network::network_trait::{Network, NetworkError};
use crate::packet::connack_packet::ConnackPacket;
use crate::packet::connect_packet::ConnectPacket;
use crate::packet::disconnect_packet::DisconnectPacket;
use crate::packet::mqtt_packet::Packet;
use crate::packet::puback_packet::PubackPacket;
use crate::packet::publish_packet::QualityOfService::QoS1;
use crate::packet::publish_packet::{PublishPacket, QualityOfService};
use crate::packet::reason_codes::ReasonCode;
use crate::packet::suback_packet::SubackPacket;
use crate::packet::subscription_packet::SubscriptionPacket;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::rng_generator::CountingRng;

pub struct MqttClientV5<'a, T, const MAX_PROPERTIES: usize> {
    network_driver: &'a mut T,
    buffer: &'a mut [u8],
    recv_buffer: &'a mut [u8],
    rng: CountingRng,
    config: ClientConfig<'a>,
}

impl<'a, T, const MAX_PROPERTIES: usize> MqttClientV5<'a, T, MAX_PROPERTIES>
where
    T: Network,
{
    pub fn new(network_driver: &'a mut T, buffer: &'a mut [u8], recv_buffer: &'a mut [u8], config: ClientConfig<'a>) -> Self {
        Self {
            network_driver,
            buffer,
            recv_buffer,
            rng: CountingRng(50),
            config
        }
    }

    pub async fn connect_to_broker<'b>(&'b mut self) -> Result<(), ReasonCode> {
        let mut len = {
            let mut connect = ConnectPacket::<'b, 3, 0>::clean();
            if self.config.username_flag {
                connect.add_username(& self.config.username);
            }
            if self.config.password_flag {
                connect.add_password(& self.config.password)
            }
            connect.encode(self.buffer)
        };

        self.network_driver.send(self.buffer, len).await ?;

        //connack
        let reason: u8 = {
            self.network_driver.receive(self.buffer).await?;
            let mut packet = ConnackPacket::<'b, 5>::new();
            packet.decode(&mut BuffReader::new(self.buffer));
            packet.connect_reason_code
        };

        if reason != 0x00 {
            return Err(ReasonCode::from(reason));
        } else {
            Ok(())
        }

    }

    pub async fn disconnect<'b>(&'b mut self) -> Result<(), ReasonCode> {
        let mut disconnect = DisconnectPacket::<'b, 5>::new();
        let mut len = disconnect.encode(self.buffer);
        self.network_driver.send(self.buffer, len).await?;
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
            packet.encode(self.buffer)
        };

        self.network_driver.send(self.buffer, len).await ?;


        //QoS1
        if <QualityOfService as Into<u8>>::into(self.config.qos ) == <QualityOfService as Into<u8>>::into(QoS1) {
            let reason = {
                self.network_driver.receive(self.buffer).await ?;
                let mut packet = PubackPacket::<'b, 5>::new();
                packet.decode(&mut BuffReader::new(self.buffer));
                [packet.packet_identifier, packet.reason_code as u16]
            };

            if identifier != reason[0] {
                return Err(ReasonCode::PacketIdentifierNotFound);
            }

            if reason[1] != 0 {
                return Err(ReasonCode::from(reason[1] as u8));
            }
        }
        Ok(())
    }

    // TODO - multiple topic subscribe func

    pub async fn subscribe_to_topic<'b>(&'b mut self, topic_name: &'b str) -> Result<(), ReasonCode> {
        let len = {
            let mut subs = SubscriptionPacket::<'b, 1, 1>::new();
            subs.add_new_filter(topic_name, self.config.qos);
            subs.encode(self.buffer)
        };

        self.network_driver.send(self.buffer, len).await ?;

        let reason = {
            self.network_driver.receive(self.buffer).await ?;

            let mut packet = SubackPacket::<'b, 5, 5>::new();
            packet.decode(&mut BuffReader::new(self.buffer));
            *packet.reason_codes.get(0).unwrap()
        };

        if reason != (<QualityOfService as Into<u8>>::into(self.config.qos) >> 1) {
            Err(ReasonCode::from(reason))
        } else {
            Ok(())
        }

    }

    pub async fn receive_message<'b>(&'b mut self) -> Result<&'b [u8], ReasonCode> {
        self.network_driver.receive(self.recv_buffer).await ?;
        let mut packet = PublishPacket::<'b, 5>::new();
        packet.decode(&mut BuffReader::new(self.recv_buffer));

        if (packet.fixed_header & 0x06) == <QualityOfService as Into<u8>>::into(QualityOfService::QoS1) {
            let mut puback = PubackPacket::<'b, 5>::new();
            puback.packet_identifier = packet.packet_identifier;
            puback.reason_code = 0x00;
            {
                let len = puback.encode(self.buffer);
                self.network_driver.send(self.buffer, len).await ?;
            }
        }

        return Ok(packet.message.unwrap());
    }
}
