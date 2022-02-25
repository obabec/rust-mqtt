use crate::packet::publish_packet::PublishPacket;
use crate::network::network_trait::Network;

struct MqttClientV5<T: Network> {
    network_driver: T,
}

impl<T> MqttClientV5<T>
where
    T: Network,
{
    fn send_message(& mut self, topic_name: & str, message: & str, buffer: & mut [u8]) {
        let packet = PublishPacket::new(topic_name, message);
        self.network_driver.send()
    }

    fn receive_message(& mut self) {

    }
}
