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
    pub async fn send_message(&'a mut self, topic_name: & str, message: & str, qos: QualityOfService) -> Result<(), NetworkError> {
        //connect
        self.network_driver.create_connection().await ?;

        let mut connect = ConnectPacket::<3, 0>::clean();
        let mut len = connect.encode(self.buffer);

        self.network_driver.send(self.buffer, len).await ?;
        //connack
        let connack: ConnackPacket<MAX_PROPERTIES> = self.receive::<ConnackPacket<MAX_PROPERTIES>>().await ?;
        if connack.connect_reason_code != 0x00 {
            todo!();
        }

        // publish
        let mut packet = PublishPacket::<5>::new(topic_name, message);
        len = packet.encode(self.buffer);
        self.network_driver.send(self.buffer, len).await ?;

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

    pub async fn receive<P: Packet<'p>>(&'a mut self) -> Result<P, NetworkError> {
        self.network_driver.receive(self.buffer).await ?;
        let mut packet = P::new();
        packet.decode(&mut BuffReader::new(self.buffer));
        return Ok(packet);
    }

    pub async fn receive_message(&'a mut self) -> Result<(), NetworkError> {
        return Ok(());
    }
}
