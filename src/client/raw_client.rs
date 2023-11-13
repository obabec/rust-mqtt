use embedded_io_async::{Read, Write};
use heapless::Vec;
use rand_core::RngCore;

use crate::{
    encoding::variable_byte_integer::{VariableByteInteger, VariableByteIntegerDecoder},
    network::NetworkConnection,
    packet::v5::{
        connack_packet::ConnackPacket,
        connect_packet::ConnectPacket,
        disconnect_packet::DisconnectPacket,
        mqtt_packet::Packet,
        packet_type::PacketType,
        pingreq_packet::PingreqPacket,
        pingresp_packet::PingrespPacket,
        puback_packet::PubackPacket,
        publish_packet::{PublishPacket, QualityOfService},
        reason_codes::ReasonCode,
        suback_packet::SubackPacket,
        subscription_packet::SubscriptionPacket,
        unsuback_packet::UnsubackPacket,
        unsubscription_packet::UnsubscriptionPacket,
    },
    utils::{buffer_reader::BuffReader, buffer_writer::BuffWriter, types::BufferError},
};

use super::client_config::{ClientConfig, MqttVersion};

pub enum Event<'a> {
    Connack,
    Puback(u16),
    Suback(u16),
    Unsuback(u16),
    Pingresp,
    Message(&'a str, &'a [u8]),
    Disconnect(ReasonCode),
}

pub struct RawMqttClient<'a, T, const MAX_PROPERTIES: usize, R: RngCore>
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

impl<'a, T, const MAX_PROPERTIES: usize, R> RawMqttClient<'a, T, MAX_PROPERTIES, R>
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

        Ok(())
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
        qos: QualityOfService,
        retain: bool,
    ) -> Result<u16, ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let identifier: u16 = self.config.rng.next_u32() as u16;
        //self.rng.next_u32() as u16;
        let len = {
            let mut packet = PublishPacket::<'b, MAX_PROPERTIES>::new();
            packet.add_topic_name(topic_name);
            packet.add_qos(qos);
            packet.add_identifier(identifier);
            packet.add_message(message);
            packet.add_retain(retain);
            packet.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        trace!("Sending message");
        conn.send(&self.buffer[0..len.unwrap()]).await?;

        Ok(identifier)
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
    ) -> Result<u16, ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.send_message_v5(topic_name, message, qos, retain).await,
        }
    }

    async fn subscribe_to_topics_v5<'b, const TOPICS: usize>(
        &'b mut self,
        topic_names: &'b Vec<&'b str, TOPICS>,
    ) -> Result<u16, ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let identifier: u16 = self.config.rng.next_u32() as u16;
        let len = {
            let mut subs = SubscriptionPacket::<'b, TOPICS, MAX_PROPERTIES>::new();
            subs.packet_identifier = identifier;
            for topic_name in topic_names.iter() {
                subs.add_new_filter(topic_name, self.config.max_subscribe_qos);
            }
            subs.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }

        conn.send(&self.buffer[0..len.unwrap()]).await?;

        Ok(identifier)
    }

    /// Method allows client subscribe to multiple topics specified in the parameter
    /// `topic_names` on the broker specified in the `ClientConfig`. Generics `TOPICS`
    /// sets the value of the `topics_names` vector. MQTT protocol implementation
    /// is selected automatically.
    pub async fn subscribe_to_topics<'b, const TOPICS: usize>(
        &'b mut self,
        topic_names: &'b Vec<&'b str, TOPICS>,
    ) -> Result<u16, ReasonCode> {
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
    ) -> Result<u16, ReasonCode> {
        match self.config.mqtt_version {
            MqttVersion::MQTTv3 => Err(ReasonCode::UnsupportedProtocolVersion),
            MqttVersion::MQTTv5 => self.unsubscribe_from_topic_v5(topic_name).await,
        }
    }

    async fn unsubscribe_from_topic_v5<'b>(
        &'b mut self,
        topic_name: &'b str,
    ) -> Result<u16, ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }
        let conn = self.connection.as_mut().unwrap();
        let identifier = self.config.rng.next_u32() as u16;

        let len = {
            let mut unsub = UnsubscriptionPacket::<'b, 1, MAX_PROPERTIES>::new();
            unsub.packet_identifier = identifier;
            unsub.add_new_filter(topic_name);
            unsub.encode(self.buffer, self.buffer_len)
        };

        if let Err(err) = len {
            error!("[DECODE ERR]: {}", err);
            return Err(ReasonCode::BuffError);
        }
        conn.send(&self.buffer[0..len.unwrap()]).await?;

        Ok(identifier)
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

        Ok(())
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

    pub async fn poll<'b, const MAX_TOPICS: usize>(&'b mut self) -> Result<Event<'b>, ReasonCode> {
        if self.connection.is_none() {
            return Err(ReasonCode::NetworkError);
        }

        let conn = self.connection.as_mut().unwrap();

        trace!("Waiting for a packet");

        let read = { receive_packet(self.buffer, self.buffer_len, self.recv_buffer, conn).await? };

        let buf_reader = BuffReader::new(self.buffer, read);

        match PacketType::from(buf_reader.peek_u8().map_err(|_| ReasonCode::BuffError)?) {
            PacketType::Reserved
            | PacketType::Connect
            | PacketType::Subscribe
            | PacketType::Unsubscribe
            | PacketType::Pingreq => Err(ReasonCode::ProtocolError),
            PacketType::Pubrec | PacketType::Pubrel | PacketType::Pubcomp | PacketType::Auth => {
                Err(ReasonCode::ImplementationSpecificError)
            }
            PacketType::Connack => {
                let mut packet = ConnackPacket::<'b, MAX_PROPERTIES>::new();
                if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                    // if err == BufferError::PacketTypeMismatch {
                    //     let mut disc = DisconnectPacket::<'b, MAX_PROPERTIES>::new();
                    //     if disc.decode(&mut BuffReader::new(self.buffer, read)).is_ok() {
                    //         error!("Client was disconnected with reason: ");
                    //         return Err(ReasonCode::from(disc.disconnect_reason));
                    //     }
                    // }
                    error!("[DECODE ERR]: {}", err);
                    Err(ReasonCode::BuffError)
                } else if packet.connect_reason_code != 0x00 {
                    Err(ReasonCode::from(packet.connect_reason_code))
                } else {
                    Ok(Event::Connack)
                }
            }
            PacketType::Puback => {
                let reason: Result<[u16; 2], BufferError> = {
                    let mut packet = PubackPacket::<'b, MAX_PROPERTIES>::new();
                    packet
                        .decode(&mut BuffReader::new(self.buffer, read))
                        .map(|_| [packet.packet_identifier, packet.reason_code as u16])
                };

                if let Err(err) = reason {
                    error!("[DECODE ERR]: {}", err);
                    return Err(ReasonCode::BuffError);
                }

                let res = reason.unwrap();

                if res[1] != 0 {
                    return Err(ReasonCode::from(res[1] as u8));
                }

                Ok(Event::Puback(res[0]))
            }
            PacketType::Suback => {
                let reason: Result<(u16, Vec<u8, MAX_TOPICS>), BufferError> = {
                    let mut packet = SubackPacket::<'b, MAX_TOPICS, MAX_PROPERTIES>::new();
                    packet
                        .decode(&mut BuffReader::new(self.buffer, read))
                        .map(|_| (packet.packet_identifier, packet.reason_codes))
                };

                if let Err(err) = reason {
                    error!("[DECODE ERR]: {}", err);
                    return Err(ReasonCode::BuffError);
                }
                let (packet_identifier, reasons) = reason.unwrap();
                for reason_code in &reasons {
                    if *reason_code
                        != (<QualityOfService as Into<u8>>::into(self.config.max_subscribe_qos)
                            >> 1)
                    {
                        return Err(ReasonCode::from(*reason_code));
                    }
                }
                Ok(Event::Suback(packet_identifier))
            }
            PacketType::Unsuback => {
                let res: Result<u16, BufferError> = {
                    let mut packet = UnsubackPacket::<'b, 1, MAX_PROPERTIES>::new();
                    packet
                        .decode(&mut BuffReader::new(self.buffer, read))
                        .map(|_| packet.packet_identifier)
                };

                if let Err(err) = res {
                    error!("[DECODE ERR]: {}", err);
                    Err(ReasonCode::BuffError)
                } else {
                    Ok(Event::Unsuback(res.unwrap()))
                }
            }
            PacketType::Pingresp => {
                let mut packet = PingrespPacket::new();
                if let Err(err) = packet.decode(&mut BuffReader::new(self.buffer, read)) {
                    error!("[DECODE ERR]: {}", err);
                    Err(ReasonCode::BuffError)
                } else {
                    Ok(Event::Pingresp)
                }
            }
            PacketType::Publish => {
                let mut packet = PublishPacket::<'b, 5>::new();
                if let Err(err) = { packet.decode(&mut BuffReader::new(self.buffer, read)) } {
                    // if err == BufferError::PacketTypeMismatch {
                    //     let mut disc = DisconnectPacket::<'b, 5>::new();
                    //     if disc.decode(&mut BuffReader::new(self.buffer, read)).is_ok() {
                    //         error!("Client was disconnected with reason: ");
                    //         return Err(ReasonCode::from(disc.disconnect_reason));
                    //     }
                    // }
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

                Ok(Event::Message(
                    packet.topic_name.string,
                    packet.message.unwrap(),
                ))
            }
            PacketType::Disconnect => {
                let mut disc = DisconnectPacket::<'b, 5>::new();
                let res = disc.decode(&mut BuffReader::new(self.buffer, read));

                match res {
                    Ok(_) => Ok(Event::Disconnect(ReasonCode::from(disc.disconnect_reason))),
                    Err(err) => {
                        error!("[DECODE ERR]: {}", err);
                        Err(ReasonCode::BuffError)
                    }
                }
            }
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
    use crate::utils::buffer_writer::RemLenError;

    let target_len: usize;
    let mut rem_len: Result<VariableByteInteger, RemLenError>;
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
            return Err(ReasonCode::NetworkError);
        }
        i += len;
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
                return Err(ReasonCode::NetworkError);
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
        return Err(ReasonCode::BuffError);
    }

    loop {
        if writer.position == target_len + rem_len_len {
            trace!("Received packet with len: {}", (target_len + rem_len_len));
            return Ok(target_len + rem_len_len);
        }
        let len: usize = conn
            .receive(&mut recv_buffer[writer.position..writer.position + (target_len - i)])
            .await?;
        i += len;
        if let Err(_e) =
            writer.insert_ref(len, &recv_buffer[writer.position..(writer.position + i)])
        {
            error!("Error occurred during write to buffer!");
            return Err(ReasonCode::BuffError);
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
        return Err(ReasonCode::BuffError);
    }
    Ok(len)
}
