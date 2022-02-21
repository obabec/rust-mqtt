/*use rust_mqtt::packet::mqtt_packet::*;
use rust_mqtt::packet::property::*;*/
/*use heapless::Vec;
use std::fs::File;
use std::io::Read;*/

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

    /*let fl = File::open("/Users/obabec/development/school/rust-mqtt/mqtt_control_example.bin");

    let mut f = File::open("/Users/obabec/development/school/rust-mqtt/mqtt_control_example.bin").expect("no file found");
    let mut buffer: [u8; 500] = [0; 500];
    f.read(&mut buffer).expect("buffer overflow");


    //
    let mut payld = *b"xxxxx";*/
    //let packet = Packet::clean(txt, &mut payld);
    /*let mut buffer_reader = BuffReader::new(&buffer);
    packet_builder.decode_packet(& mut buffer_reader);


    let bytes: [u8; 4] = packet_builder.currentPacket.protocol_name.to_be_bytes();

    let prot = std::str::from_utf8(&bytes).unwrap();
    log::info!("Protocol name: {}", prot)*/
}
