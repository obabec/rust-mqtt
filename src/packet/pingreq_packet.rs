use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

pub struct PingreqPacket {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,
}

impl PingreqPacket {}

impl<'a> Packet<'a> for PingreqPacket {
    fn new() -> Self {
        todo!()
    }

    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);
        buff_writer.write_u8(self.fixed_header);
        buff_writer.write_variable_byte_int(0 as u32);
        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        log::error!("PingreqPacket packet does not support decode funtion on client!");
    }

    fn set_property_len(&mut self, value: u32) {
        log::error!("PINGREQ packet does not contain any properties!");
    }

    fn get_property_len(&mut self) -> u32 {
        log::error!("PINGREQ packet does not contain any properties!");
        return 0;
    }

    fn push_to_properties(&mut self, property: Property<'a>) {
        log::error!("PINGREQ packet does not contain any properties!");
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
