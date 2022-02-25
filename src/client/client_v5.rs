use core::future::Future;
use crate::network::network_trait::{Network, NetworkError};
use crate::packet::connack_packet::ConnackPacket;
use crate::packet::connect_packet::ConnectPacket;
use crate::packet::disconnect_packet::DisconnectPacket;
use crate::packet::mqtt_packet::Packet;
use crate::packet::publish_packet::{PublishPacket, QualityOfService};
use crate::packet::publish_packet::QualityOfService::QoS1;
use crate::utils::buffer_reader::BuffReader;

pub struct MqttClientV5<'a, T, const MAX_PROPERTIES: usize> {
    network_driver: &'a mut T,
    buffer: &'a mut [u8]
}

impl<'a, T, const MAX_PROPERTIES: usize> MqttClientV5<'a, T, MAX_PROPERTIES>
where
    T: Network
{
    pub fn new(network_driver: &'a mut T, buffer: &'a mut [u8]) -> Self {
        Self {
            network_driver,
            buffer
        }
    }
    // connect -> connack -> publish -> QoS ? -> disconn
    pub async fn send_message(&'a mut self, topic_name: & str, message: & str, qos: QualityOfService) -> impl Future<Output = Result<(), NetworkError>> {
        async move {
            let mut len = {
                let mut connect = ConnectPacket::<3, 0>::clean();
                connect.encode(self.buffer)
            };

            self.network_driver.send(self.buffer, len).await?;

            //connack
            let connack = {
                let connack = self.receive().await?;
                let mut packet = ConnackPacket::new();
                packet.decode(&mut BuffReader::new(self.buffer));
                packet
            };

            if connack.connect_reason_code != 0x00 {
                todo!();
            }

            // publish

            len = {
                let mut packet = PublishPacket::<5>::new(topic_name, message);
                packet.encode(self.buffer)
            };

            self.network_driver.send(self.buffer, len).await?;


            //QoS1
            if <QualityOfService as Into<u8>>::into(qos) == <QualityOfService as Into<u8>>::into(QoS1) {
                todo!();
            }

            //Disconnect
            let mut disconnect = DisconnectPacket::<5>::new();
            len = disconnect.encode(self.buffer);
            self.network_driver.send(self.buffer, len);
            Ok(())
        }
    }

    pub async fn receive(&'a mut self) -> Result<(), NetworkError> {
        self.network_driver.receive(self.buffer).await ?;
        Ok(())
    }

    pub async fn receive_message(&'a mut self) -> Result<(), NetworkError> {
        return Ok(());
    }
}
