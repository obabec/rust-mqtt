use rust_mqtt::client::client_v5::MqttClientV5;
use rust_mqtt::network::network_trait::Network;
use rust_mqtt::packet::connect_packet::ConnectPacket;
use rust_mqtt::packet::mqtt_packet::Packet;
use rust_mqtt::packet::publish_packet::PublishPacket;
use rust_mqtt::packet::subscription_packet::SubscriptionPacket;
use rust_mqtt::tokio_network::TokioNetwork;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();

    /*let mut pckt: SubscriptionPacket<1, 0> = SubscriptionPacket::new();
    let mut res = vec![0; 140];
    let lnsub = pckt.encode(&mut res);
    println!("{:02X?}", &res[0..lnsub]);
    let mut res2 = vec![0; 260];

    let mut pblsh = PublishPacket::<0>::new(x);
    let lnpblsh = pblsh.encode(&mut res2);
    println!("{:02X?}", &res2[0..lnpblsh]);
    log::info!("xxx");

    let mut res3 = vec![0; 260];
    let mut cntrl = ConnectPacket::<3, 0>::clean();
    let lncntrl = cntrl.encode(&mut res3);
    println!("{:02X?}", &res3[0..lncntrl]);
    log::info!("xxx");*/
    let mut ip: [u8; 4] = [37, 205, 11, 180];
    let mut port: u16 = 1883;
    let mut tokio_network: TokioNetwork = TokioNetwork::new(ip, port);
    tokio_network.create_connection().await;
    let mut res2 = vec![0; 260];
    let client = MqttClientV5::new(&mut tokio_network, &mut res2);
    let mut x = b"hello world";
}
