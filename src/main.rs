use rust_mqtt::packet::mqttpacket::Packet;

fn main() {
    
    env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .format_timestamp_nanos()
    .init();

    let l: u8 = 1;
    let y: u32 = 2;
    let z: u16 = 3;
    let p: u32 = 4;
    let text: &'a [u8] = "abcde";
    let payld: &'a [u8] = "This is payload";

    let x = Packet::new( l, y, z, p, text, payld );
    log::info!("Hello world");
    x.encode();
    x.get_reason_code();
}