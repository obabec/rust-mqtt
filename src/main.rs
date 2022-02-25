use rust_mqtt::packet::connect_packet::ConnectPacket;
use rust_mqtt::packet::mqtt_packet::Packet;
use rust_mqtt::packet::publish_packet::PublishPacket;
use rust_mqtt::packet::subscription_packet::SubscriptionPacket;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp_nanos()
        .init();

    let mut pckt: SubscriptionPacket<1, 0> = SubscriptionPacket::new();
    let mut res = vec![0; 140];
    let lnsub = pckt.encode(&mut res);
    println!("{:02X?}", &res[0..lnsub]);
    let mut res2 = vec![0; 260];
    let mut x = b"hello world";
    let mut pblsh = PublishPacket::<0>::new(x);
    let lnpblsh = pblsh.encode(&mut res2);
    println!("{:02X?}", &res2[0..lnpblsh]);
    log::info!("xxx");

    let mut res3 = vec![0; 260];
    let mut cntrl = ConnectPacket::<3, 0>::clean();
    let lncntrl = cntrl.encode(&mut res3);
    println!("{:02X?}", &res3[0..lncntrl]);
    log::info!("xxx");
}
