use crate::packet::mqtt_packet::Packet;
use crate::utils::buffer_reader::BuffReader;
use crate::utils::buffer_writer::BuffWriter;

use super::packet_type::PacketType;
use super::property::Property;

pub struct PingrespPacket {
    // 7 - 4 mqtt control packet type, 3-0 flagy
    pub fixed_header: u8,
    // 1 - 4 B lenght of variable header + len of payload
    pub remain_len: u32,
}

impl<'a> PingrespPacket {
    pub fn decode_pingresp_packet(&mut self, buff_reader: &mut BuffReader<'a>) {
        if self.decode_fixed_header(buff_reader) != (PacketType::Pingresp).into() {
            log::error!("Packet you are trying to decode is not PUBACK packet!");
            return;
        }
    }
}

impl<'a> Packet<'a> for PingrespPacket {
    fn encode(&mut self, buffer: &mut [u8]) -> usize {
        let mut buff_writer = BuffWriter::new(buffer);
        buff_writer.write_u8(self.fixed_header);
        buff_writer.write_variable_byte_int(0 as u32);
        return buff_writer.position;
    }

    fn decode(&mut self, buff_reader: &mut BuffReader<'a>) {
        self.decode_pingresp_packet(buff_reader);
    }

    fn set_property_len(&mut self, value: u32) {
        log::error!("PINGRESP packet does not contain any properties!");
    }

    fn get_property_len(&mut self) -> u32 {
        log::error!("PINGRESP packet does not contain any properties!");
        return 0;
    }

    fn push_to_properties(&mut self, property: Property<'a>) {
        log::error!("PINGRESP packet does not contain any properties!");
    }

    fn set_fixed_header(&mut self, header: u8) {
        self.fixed_header = header;
    }

    fn set_remaining_len(&mut self, remaining_len: u32) {
        self.remain_len = remaining_len;
    }
}
