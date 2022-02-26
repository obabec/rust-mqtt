use core::future::Future;

use crate::network::network_trait::{Network, NetworkError};
use crate::packet::connack_packet::ConnackPacket;
use crate::packet::connect_packet::ConnectPacket;
use crate::packet::disconnect_packet::DisconnectPacket;
use crate::packet::mqtt_packet::Packet;
use crate::packet::publish_packet::QualityOfService::QoS1;
use crate::packet::publish_packet::{PublishPacket, QualityOfService};
use crate::packet::suback_packet::SubackPacket;
use crate::packet::subscription_packet::SubscriptionPacket;
use crate::utils::buffer_reader::BuffReader;

pub struct MqttClientV5<'a, T, const MAX_PROPERTIES: usize> {
    network_driver: &'a mut T,
    buffer: &'a mut [u8],
}

impl<'a, T, const MAX_PROPERTIES: usize> MqttClientV5<'a, T, MAX_PROPERTIES>
where
    T: Network,
{
    pub fn new(network_driver: &'a mut T, buffer: &'a mut [u8]) -> Self {
        Self {
            network_driver,
            buffer,
        }
    }

    pub async fn connect_to_broker<'b>(&'b mut self) -> Result<(), NetworkError> {
        let mut len = {
            let mut connect = ConnectPacket::<'b, 3, 0>::clean();
            connect.encode(self.buffer)
        };

        self.network_driver.send(self.buffer, len).await?;

        //connack
        let reason: u8 = {
            self.network_driver.receive(self.buffer).await?;
            let mut packet = ConnackPacket::<'b, 5>::new();
            packet.decode(&mut BuffReader::new(self.buffer));
            packet.connect_reason_code
        };

        if reason != 0x00 {
            Err(NetworkError::Connection)
        } else {
            Ok(())
        }

    }

    pub async fn disconnect<'b>(&'b mut self) -> Result<(), NetworkError> {
        let mut disconnect = DisconnectPacket::<'b, 5>::new();
        let mut len = disconnect.encode(self.buffer);
        self.network_driver.send(self.buffer, len).await?;
        Ok(())
    }


    // connect -> connack -> publish -> QoS ? -> disconn
    pub async fn send_message<'b>(
        &'b mut self,
        topic_name: &'b str,
        message: &'b str,
        qos: QualityOfService,
    ) -> Result<(), NetworkError> {
        // publish
        let len = {
            let mut packet = PublishPacket::<'b, 5>::new();
            packet.add_topic_name(topic_name);
            packet.add_message(message.as_bytes());
            packet.encode(self.buffer)
        };

        self.network_driver.send(self.buffer, len).await?;

        //QoS1
        if <QualityOfService as Into<u8>>::into(qos) == <QualityOfService as Into<u8>>::into(QoS1) {
            todo!();
        }
        Ok(())
    }

    pub async fn subscribe_to_topic<'b>(&'b mut self, topic_name: &'b str) -> Result<(), NetworkError> {
        let len = {
            let mut subs = SubscriptionPacket::<'b, 1, 1>::new();
            subs.add_new_filter(topic_name);
            subs.encode(self.buffer)
        };
        let xx: [u8; 14] = (self.buffer[0..14]).try_into().unwrap();
        log::info!("{:x?}", xx);
        self.network_driver.send(self.buffer, len).await?;

        let reason = {
            self.network_driver.receive(self.buffer).await?;
            let mut packet = SubackPacket::<'b, 5, 5>::new();
            packet.decode(&mut BuffReader::new(self.buffer));
            *packet.reason_codes.get(0).unwrap()
        };

        if reason > 1 {
            Err(NetworkError::Unknown)
        } else {
            Ok(())
        }

    }

    pub async fn receive_message<'b>(&'b mut self) -> Result<&'b [u8], NetworkError> {
        self.network_driver.receive(self.buffer).await?;
        let mut packet = PublishPacket::<'b, 5>::new();
        packet.decode(&mut BuffReader::new(self.buffer));
        return Ok(packet.message.unwrap());
    }
}
