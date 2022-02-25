use crate::packet::publish_packet::{PublishPacket, QualityOfService};
use crate::network::network_trait::{Network, NetworkError};
use crate::packet::connack_packet::ConnackPacket;
use crate::packet::connect_packet::ConnectPacket;
use crate::packet::disconnect_packet::DisconnectPacket;
use crate::packet::mqtt_packet::Packet;
use crate::packet::publish_packet::QualityOfService::QoS1;
use crate::utils::buffer_reader::BuffReader;

pub struct MqttClientV5<T, const MAX_PROPERTIES: usize> {
    network_driver: T,
}

impl<T, const MAX_PROPERTIES: usize> MqttClientV5<T, MAX_PROPERTIES>
where
    T: Network
{
    pub fn new(network_driver: T) -> Self {
        Self {
            network_driver,
        }
    }
    // connect -> connack -> publish -> QoS ? -> disconn
    pub async fn send_message(& mut self, topic_name: & str, message: & str, buffer: & mut [u8], qos: QualityOfService) -> Result<(), NetworkError> {
        //connect
        self.network_driver.create_connection() ?;

        let mut connect = ConnectPacket::clean();
        let mut len = connect.encode(buffer);
        self.network_driver.send(buffer, len).await ?;
        //connack
        let connack: ConnackPacket<MAX_PROPERTIES> = self.receive::<ConnackPacket<MAX_PROPERTIES>>(buffer).await ?;
        if connack.connect_reason_code != 0x00 {
            todo!();
        }

        // publish
        let mut packet = PublishPacket::new(topic_name, message);
        len = packet.encode(buffer);
        let result = self.network_driver.send(buffer, len).await ?;

        //QoS1
        if qos.into() == QoS1.into() {
            todo!();
        }

        //Disconnect
        let mut disconnect = DisconnectPacket::new();
        len = disconnect.encode(buffer);
        self.network_driver.send(buffer, len);
        return result;
    }

    pub async fn receive<P: Packet<'a>>(& mut self, buffer: & mut [u8]) -> Result<P, ()> {
        self.network_driver.receive(buffer).await ?;
        let mut packet = P::new();
        packet.decode(&mut BuffReader::new(buffer));
        return Ok(packet);
    }

    pub async fn receive_message(& mut self, buffer: & mut [u8]) -> Result<(), NetworkError> {

    }
}
