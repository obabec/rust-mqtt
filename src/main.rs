use rust_mqtt::packet::mqtt_packet::*;
use rust_mqtt::packet::packet_type::PacketType;
use rust_mqtt::packet::packet_builder::PacketBuilder;
use rust_mqtt::encoding::variable_byte_integer::VariableByteIntegerEncoder;
use rust_mqtt::encoding::variable_byte_integer::VariableByteIntegerDecoder;
use rust_mqtt::packet::property::*;
use rust_mqtt::utils::buffer_reader::BuffReader;
use heapless::Vec;
use std::fs::File;
use std::io::Read;

fn main() {
    env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .format_timestamp_nanos()
    .init();

    let fl = File::open("/Users/obabec/development/school/rust-mqtt/mqtt_control_example.bin");

    let mut f = File::open("/Users/obabec/development/school/rust-mqtt/mqtt_control_example.bin").expect("no file found");
    let mut buffer: [u8; 500] = [0; 500];
    f.read(&mut buffer).expect("buffer overflow");


    let mut txt = Vec::new();
    let mut payld = *b"xxxxx";
    let packet = Packet::clean(txt, &mut payld);
    let mut packet_builder = PacketBuilder::new(packet);
    let mut buffer_reader = BuffReader::new(&buffer);
    packet_builder.decode_packet(& mut buffer_reader);

    
    let bytes: [u8; 4] = packet_builder.currentPacket.protocol_name.to_be_bytes();

    let prot = std::str::from_utf8(&bytes).unwrap();
    log::info!("Protocol name: {}", prot)
}

/*fn test(tst: &str) {
    log::info!("xx");
    log::info!("Prvni: {}", )
}*/