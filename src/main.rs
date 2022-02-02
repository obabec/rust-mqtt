use rust_mqtt::packet::mqtt_packet::Packet;
use rust_mqtt::packet::packet_type::PacketType;

fn main() {
    env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .format_timestamp_nanos()
    .init();

    let l: u8 = 1;
    let y: u32 = 2;
    let z: u16 = 3;
    let p: u32 = 4;

    let mut txt = *b"abcde";
    let mut payld = *b"xxxxx";
    let f = PacketType::Reserved;
    let o: u8 = f.into();
    log::info!("Enum val: {}", o);
    let x = Packet::new( l, y, z, p, &mut txt, &mut payld );
    log::info!("Hello world");
    x.encode();
    x.get_reason_code();
}